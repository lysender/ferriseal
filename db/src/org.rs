use async_trait::async_trait;
use deadpool_diesel::sqlite::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use memo::client::OrgDto;
use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{
    DbInteractSnafu, DbPoolSnafu, DbQuerySnafu, MaxorgsReachedSnafu, ValidationSnafu,
};
use crate::schema::orgs::{self, dsl};
use crate::state::AppState;
use memo::{utils::generate_id, validators::flatten_errors};

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::orgs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Org {
    pub id: String,
    pub name: String,
    pub default_bucket_id: Option<String>,
    pub status: String,
    pub admin: Option<i32>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewOrg {
    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::anyname"))]
    pub name: String,

    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::uuid"))]
    pub default_bucket_id: Option<String>,

    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::status"))]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Validate, AsChangeset)]
#[diesel(table_name = crate::schema::orgs)]
pub struct UpdateOrg {
    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::anyname"))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::uuid"))]
    pub default_bucket_id: Option<Option<String>>,

    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::status"))]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OrgDefaultBucket {
    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "memo::validators::uuid"))]
    pub default_bucket_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::orgs)]
pub struct UpdateOrgBucket {
    #[diesel(treat_none_as_null = true)]
    pub default_bucket_id: Option<String>,
}

impl From<OrgDto> for Org {
    fn from(dto: OrgDto) -> Self {
        Org {
            id: dto.id,
            name: dto.name,
            default_bucket_id: dto.default_bucket_id,
            status: dto.status,
            admin: if dto.admin { Some(1) } else { Some(0) },
            created_at: dto.created_at,
        }
    }
}

impl From<Org> for OrgDto {
    fn from(client: Org) -> Self {
        OrgDto {
            id: client.id,
            name: client.name,
            default_bucket_id: client.default_bucket_id,
            status: client.status,
            admin: match client.admin {
                Some(1) => true,
                _ => false,
            },
            created_at: client.created_at,
        }
    }
}

// Can't have too many orgs
const MAX_orgs: i32 = 10;

pub async fn create_client(state: &AppState, data: &NewOrg, admin: bool) -> Result<Org> {
    let valid_res = data.validate();
    ensure!(
        valid_res.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&valid_res.unwrap_err()),
        }
    );

    // Limit the number of orgs because we are poor!
    let count = state.db.orgs.count().await?;
    ensure!(count < MAX_orgs as i64, MaxorgsReachedSnafu,);

    state.db.orgs.create(data, admin).await
}

pub async fn update_client(state: &AppState, id: &str, data: &UpdateOrg) -> Result<bool> {
    let valid_res = data.validate();
    ensure!(
        valid_res.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&valid_res.unwrap_err()),
        }
    );

    // We can't tell whether we are setting default bucket to null or skipping it
    // Will just use a separate function for that
    if let Some(bucket_id) = data.default_bucket_id.clone() {
        if let Some(bid) = bucket_id {
            let bucket = state.db.buckets.get(&bid).await?;
            ensure!(
                bucket.is_some(),
                ValidationSnafu {
                    msg: "Default bucket not found".to_string(),
                }
            );
        }
    }

    state.db.orgs.update(id, data).await
}

pub async fn delete_client(state: &AppState, id: &str) -> Result<()> {
    let Some(client) = state.db.orgs.get(id).await? else {
        return ValidationSnafu {
            msg: "Org not found".to_string(),
        }
        .fail();
    };

    ensure!(
        !client.admin,
        ValidationSnafu {
            msg: "Cannot delete admin client".to_string(),
        }
    );

    let bucket_count = state.db.buckets.count_by_client(id).await?;
    ensure!(
        bucket_count == 0,
        ValidationSnafu {
            msg: "Org still has buckets".to_string(),
        }
    );

    let users_count = state.db.users.count_by_client(id).await?;
    ensure!(
        users_count == 0,
        ValidationSnafu {
            msg: "Org still has users".to_string(),
        }
    );

    state.db.orgs.delete(id).await
}

#[async_trait]
pub trait OrgRepoable: Send + Sync {
    async fn list(&self, client_id: Option<String>) -> Result<Vec<Org>>;

    async fn find_admin(&self) -> Result<Option<Org>>;

    async fn create(&self, data: &NewOrg, admin: bool) -> Result<Org>;

    async fn get(&self, id: &str) -> Result<Option<OrgDto>>;

    async fn update(&self, id: &str, data: &UpdateOrg) -> Result<bool>;

    async fn find_by_name(&self, name: &str) -> Result<Option<OrgDto>>;

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
        let admin = if admin { Some(1) } else { Some(0) };
        let client = Org {
            id: generate_id(),
            name: data.name.clone(),
            default_bucket_id: data.default_bucket_id.clone(),
            status: data.status.clone(),
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
        default_bucket_id: None,
        status: "active".to_string(),
        admin: None,
        created_at: today,
    }
}

#[cfg(test)]
pub fn create_test_admin_client() -> Org {
    let today = chrono::Utc::now().timestamp();
    Org {
        id: TEST_ADMIN_CLIENT_ID.to_string(),
        name: "Test Admin Org".to_string(),
        default_bucket_id: None,
        status: "active".to_string(),
        admin: Some(1),
        created_at: today,
    }
}

#[cfg(test)]
pub fn create_test_new_client() -> Org {
    let today = chrono::Utc::now().timestamp();
    Org {
        id: TEST_NEW_CLIENT_ID.to_string(),
        name: "Test New Org".to_string(),
        default_bucket_id: None,
        status: "active".to_string(),
        admin: None,
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
