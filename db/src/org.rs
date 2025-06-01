use async_trait::async_trait;
use deadpool_diesel::sqlite::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use dto::org::OrgDto;
use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu, ValidationSnafu};
use crate::schema::orgs::{self, dsl};
use vault::utils::generate_id;

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::orgs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Org {
    pub id: String,
    pub name: String,
    pub admin: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewOrg {
    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "vault::validators::anyname"))]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Validate, AsChangeset)]
#[diesel(table_name = crate::schema::orgs)]
pub struct UpdateOrg {
    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "vault::validators::anyname"))]
    pub name: Option<String>,
}

impl From<OrgDto> for Org {
    fn from(dto: OrgDto) -> Self {
        Org {
            id: dto.id,
            name: dto.name,
            admin: dto.admin,
            created_at: dto.created_at,
        }
    }
}

impl From<Org> for OrgDto {
    fn from(client: Org) -> Self {
        OrgDto {
            id: client.id,
            name: client.name,
            admin: client.admin,
            created_at: client.created_at,
        }
    }
}

#[async_trait]
pub trait OrgRepoable: Send + Sync {
    async fn list(&self, client_id: Option<String>) -> Result<Vec<Org>>;

    async fn find_admin(&self) -> Result<Option<Org>>;

    async fn create(&self, data: &NewOrg, admin: bool) -> Result<Org>;

    async fn get(&self, id: &str) -> Result<Option<OrgDto>>;

    async fn update(&self, id: &str, data: &UpdateOrg) -> Result<bool>;

    async fn find_by_name(&self, name: &str) -> Result<Option<Org>>;

    async fn count(&self) -> Result<i64>;

    async fn delete(&self, id: &str) -> Result<()>;
}

pub struct OrgRepo {
    db_pool: Pool,
}

impl OrgRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl OrgRepoable for OrgRepo {
    async fn list(&self, client_id: Option<String>) -> Result<Vec<Org>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let client_id_copy = client_id.clone();
        let select_res = db
            .interact(move |conn| {
                let mut query = dsl::orgs.into_boxed();
                if let Some(cid) = client_id_copy {
                    query = query.filter(dsl::id.eq(cid));
                }

                query
                    .select(Org::as_select())
                    .order(dsl::name.asc())
                    .load::<Org>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(items)
    }

    async fn find_admin(&self) -> Result<Option<Org>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::orgs
                    .filter(dsl::admin.eq(Some(1)))
                    .select(Org::as_select())
                    .first::<Org>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let item = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(item)
    }

    async fn create(&self, data: &NewOrg, admin: bool) -> Result<Org> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Org name must be unique
        let existing = self.find_by_name(&data.name).await?;
        ensure!(
            existing.is_none(),
            ValidationSnafu {
                msg: "Org name already exists".to_string(),
            }
        );

        let today = chrono::Utc::now().timestamp();
        let client = Org {
            id: generate_id(),
            name: data.name.clone(),
            admin,
            created_at: today,
        };

        let client_copy = client.clone();
        let insert_res = db
            .interact(move |conn| {
                diesel::insert_into(orgs::table)
                    .values(&client_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = insert_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(client)
    }

    async fn get(&self, id: &str) -> Result<Option<OrgDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let cid = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::orgs
                    .find(cid)
                    .select(Org::as_select())
                    .first::<Org>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let item = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(item.map(|item| item.into()))
    }

    async fn update(&self, id: &str, data: &UpdateOrg) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Org name must be unique
        if let Some(name) = data.name.clone() {
            if let Some(existing) = self.find_by_name(&name).await? {
                ensure!(
                    &existing.id == id,
                    ValidationSnafu {
                        msg: "Org name already exists".to_string(),
                    }
                );
            }
        }

        let id = id.to_string();
        let data_copy = data.clone();
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::orgs)
                    .filter(dsl::id.eq(id.as_str()))
                    .set(data_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let item = update_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(item > 0)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<OrgDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let name_copy = name.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::orgs
                    .filter(dsl::name.eq(name_copy.as_str()))
                    .select(Org::as_select())
                    .first::<Org>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let item = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(item.map(|item| item.into()))
    }

    async fn count(&self) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| dsl::orgs.select(count_star()).get_result::<i64>(conn))
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(count)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::orgs.filter(dsl::id.eq(id.as_str()))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(test)]
pub const TEST_CLIENT_ID: &'static str = "0196d19e01b1745980a8419edd88e3d1";

#[cfg(test)]
pub const TEST_ADMIN_CLIENT_ID: &'static str = "0196d1a2784a72959c97eef5dbc69dc7";

#[cfg(test)]
pub const TEST_NEW_CLIENT_ID: &'static str = "0196d1a2784a72959c97eef5dbc69dc7";

#[cfg(test)]
pub struct OrgTestRepo {}

#[cfg(test)]
pub fn create_test_client() -> Org {
    let today = chrono::Utc::now().timestamp();
    Org {
        id: TEST_CLIENT_ID.to_string(),
        name: "Test Org".to_string(),
        admin: false,
        created_at: today,
    }
}

#[cfg(test)]
pub fn create_test_admin_client() -> Org {
    let today = chrono::Utc::now().timestamp();
    Org {
        id: TEST_ADMIN_CLIENT_ID.to_string(),
        name: "Test Admin Org".to_string(),
        admin: true,
        created_at: today,
    }
}

#[cfg(test)]
pub fn create_test_new_client() -> Org {
    let today = chrono::Utc::now().timestamp();
    Org {
        id: TEST_NEW_CLIENT_ID.to_string(),
        name: "Test New Org".to_string(),
        admin: false,
        created_at: today,
    }
}

#[cfg(test)]
#[async_trait]
impl OrgRepoable for OrgTestRepo {
    async fn list(&self, client_id: Option<String>) -> Result<Vec<Org>> {
        let client1 = create_test_client();
        let client2 = create_test_admin_client();
        let orgs = vec![client1, client2];
        match client_id {
            Some(cid) => {
                let filtered: Vec<Org> =
                    orgs.into_iter().filter(|x| x.id.as_str() == cid).collect();
                Ok(filtered)
            }
            None => Ok(orgs),
        }
    }

    async fn find_admin(&self) -> Result<Option<Org>> {
        Ok(Some(create_test_admin_client()))
    }

    async fn create(&self, _data: &NewOrg, _admin: bool) -> Result<Org> {
        Ok(create_test_new_client())
    }

    async fn get(&self, id: &str) -> Result<Option<OrgDto>> {
        let orgs = self.list(None).await?;
        let found = orgs.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _id: &str, _data: &UpdateOrg) -> Result<bool> {
        Ok(true)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<OrgDto>> {
        let orgs = self.list(None).await?;
        let found = orgs.into_iter().find(|x| x.name.as_str() == name);
        Ok(found.map(|x| x.into()))
    }

    async fn count(&self) -> Result<i64> {
        Ok(2)
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
