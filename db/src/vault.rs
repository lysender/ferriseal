use async_trait::async_trait;

use deadpool_diesel::sqlite::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use validator::Validate;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::vaults::{self, dsl};
use dto::vault::VaultDto;
use vault::utils::generate_id;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize)]
#[diesel(table_name = crate::schema::vaults)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Vault {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub test_cipher: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewVault {
    #[validate(length(min = 1, max = 50))]
    pub name: String,

    #[validate(length(min = 1, max = 250))]
    pub test_cipher: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ListVaultsParams {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}

impl From<VaultDto> for Vault {
    fn from(dto: VaultDto) -> Self {
        Vault {
            id: dto.id,
            org_id: dto.org_id,
            name: dto.name,
            test_cipher: dto.test_cipher,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }
}

impl From<Vault> for VaultDto {
    fn from(vault: Vault) -> Self {
        VaultDto {
            id: vault.id,
            org_id: vault.org_id,
            name: vault.name,
            test_cipher: vault.test_cipher,
            created_at: vault.created_at,
            updated_at: vault.updated_at,
        }
    }
}

#[async_trait]
pub trait VaultRepoable: Send + Sync {
    async fn list(&self, org_id: &str) -> Result<Vec<VaultDto>>;

    async fn create(&self, org_id: &str, data: &NewVault) -> Result<VaultDto>;

    async fn get(&self, id: &str) -> Result<Option<VaultDto>>;

    async fn count_by_org(&self, org_id: &str) -> Result<i64>;

    async fn delete(&self, id: &str) -> Result<()>;

    async fn test_read(&self) -> Result<()>;
}

pub struct VaultRepo {
    db_pool: Pool,
}

impl VaultRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl VaultRepoable for VaultRepo {
    async fn list(&self, org_id: &str) -> Result<Vec<VaultDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let org_id = org_id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::vaults
                    .filter(dsl::org_id.eq(&org_id))
                    .select(Vault::as_select())
                    .order(dsl::name.asc())
                    .load::<Vault>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "vaults".to_string(),
        })?;

        let dtos: Vec<VaultDto> = items.into_iter().map(|item| item.into()).collect();
        Ok(dtos)
    }

    async fn create(&self, org_id: &str, data: &NewVault) -> Result<VaultDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;
        let data_copy = data.clone();
        let today = chrono::Utc::now().timestamp();
        let vault = Vault {
            id: generate_id(),
            org_id: org_id.to_string(),
            name: data_copy.name,
            test_cipher: data_copy.test_cipher,
            created_at: today.clone(),
            updated_at: today,
        };

        let vault_copy = vault.clone();
        let insert_res = db
            .interact(move |conn| {
                diesel::insert_into(vaults::table)
                    .values(&vault_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = insert_res.context(DbQuerySnafu {
            table: "vaults".to_string(),
        })?;

        Ok(vault.into())
    }

    async fn get(&self, id: &str) -> Result<Option<VaultDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let bid = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::vaults
                    .find(bid)
                    .select(Vault::as_select())
                    .first::<Vault>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let item = select_res.context(DbQuerySnafu {
            table: "vaults".to_string(),
        })?;

        Ok(item.map(|item| item.into()))
    }

    async fn count_by_org(&self, org_id: &str) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let cid = org_id.to_string();
        let count_res = db
            .interact(move |conn| {
                dsl::vaults
                    .filter(dsl::org_id.eq(cid.as_str()))
                    .select(count_star())
                    .get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "vaults".to_string(),
        })?;

        Ok(count)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let vault_id = id.to_string();
        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::vaults.filter(dsl::id.eq(vault_id.as_str()))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "vaults".to_string(),
        })?;

        Ok(())
    }

    async fn test_read(&self) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let selected_res = db
            .interact(move |conn| {
                dsl::vaults
                    .select(Vault::as_select())
                    .first::<Vault>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = selected_res.context(DbQuerySnafu {
            table: "vaults".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(test)]
pub const TEST_VAULT_ID: &'static str = "0196d1bbc22f79c89cdbc8beced0d2f0";

#[cfg(test)]
pub fn create_test_vault() -> VaultDto {
    use crate::org::TEST_ORG_ID;
    let today = chrono::Utc::now().timestamp();

    VaultDto {
        id: TEST_VAULT_ID.to_string(),
        org_id: TEST_ORG_ID.to_string(),
        name: "test-vault".to_string(),
        test_cipher: "test-cipher".to_string(),
        created_at: today,
        updated_at: today,
    }
}

#[cfg(test)]
pub struct VaultTestRepo {}

#[cfg(test)]
#[async_trait]
impl VaultRepoable for VaultTestRepo {
    async fn list(&self, org_id: &str) -> Result<Vec<VaultDto>> {
        let vault = create_test_vault();
        let vaults = vec![vault];
        let filtered = vaults
            .into_iter()
            .filter(|x| x.org_id.as_str() == org_id)
            .collect();
        Ok(filtered)
    }

    async fn create(&self, _org_id: &str, _data: &NewVault) -> Result<VaultDto> {
        Err("No supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<VaultDto>> {
        let vault = create_test_vault();
        let vaults = vec![vault];
        let found = vaults.into_iter().find(|x| x.id.as_str() == id);
        Ok(found)
    }

    async fn count_by_org(&self, org_id: &str) -> Result<i64> {
        let vaults = self.list(org_id).await?;
        Ok(vaults.len() as i64)
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }

    async fn test_read(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_vault() {
        let data = NewVault {
            name: "hello-world".to_string(),
            test_cipher: "hello-world".to_string(),
        };
        assert!(data.validate().is_ok());

        let data = NewVault {
            name: "hello_world".to_string(),
            test_cipher: "".to_string(),
        };
        assert!(data.validate().is_err());

        let data = NewVault {
            name: "".to_string(),
            test_cipher: "hello-world".to_string(),
        };
        assert!(data.validate().is_err());
    }
}
