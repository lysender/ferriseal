use async_trait::async_trait;

use deadpool_diesel::sqlite::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu, ValidationSnafu};
use crate::schema::entries::{self, dsl};
use crate::vault::Vault;
use dto::entry::EntryDto;
use dto::pagination::PaginatedDto;
use vault::validators::flatten_errors;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize)]
#[diesel(table_name = crate::schema::entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Entry {
    pub id: String,
    pub vault_id: String,
    pub label: String,
    pub cipher_username: Option<String>,
    pub cipher_password: Option<String>,
    pub cipher_notes: Option<String>,
    pub cipher_extra_notes: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct EntryPayload {
    pub label: String,
    pub cipher_username: Option<String>,
    pub cipher_password: Option<String>,
    pub cipher_notes: Option<String>,
    pub cipher_extra_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ListEntriesParams {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}

/// Convert EntryDto to Entry
impl From<EntryDto> for Entry {
    fn from(entry: EntryDto) -> Self {
        Self {
            id: entry.id,
            vault_id: entry.vault_id,
            label: entry.label,
            cipher_username: entry.cipher_username,
            cipher_password: entry.cipher_password,
            cipher_notes: entry.cipher_notes,
            cipher_extra_notes: entry.cipher_extra_notes,
            status: entry.status,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
        }
    }
}

/// Convert Entry to EntryDto
impl From<Entry> for EntryDto {
    fn from(entry: Entry) -> Self {
        Self {
            id: entry.id,
            vault_id: entry.vault_id,
            label: entry.label,
            cipher_username: entry.cipher_username,
            cipher_password: entry.cipher_password,
            cipher_notes: entry.cipher_notes,
            cipher_extra_notes: entry.cipher_extra_notes,
            status: entry.status,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
        }
    }
}

const MAX_PER_PAGE: i32 = 50;

#[async_trait]
pub trait EntryRepoable: Send + Sync {
    async fn list(&self, vault: &Vault, params: &ListEntriesParams) -> Result<PaginatedDto<Entry>>;

    async fn create(&self, entry_dto: EntryDto) -> Result<Entry>;

    async fn get(&self, id: &str) -> Result<Option<Entry>>;

    async fn count_by_vault(&self, vault_id: &str) -> Result<i64>;

    async fn delete(&self, id: &str) -> Result<()>;
}

pub struct EntryRepo {
    db_pool: Pool,
}

impl EntryRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    pub async fn listing_count(&self, vault_id: &str, params: &ListEntriesParams) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let vid = vault_id.to_string();
        let params_copy = params.clone();

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::entries.into_boxed();
                query = query.filter(dsl::vault_id.eq(vid.as_str()));
                if let Some(keyword) = params_copy.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(dsl::label.like(pattern));
                    }
                }
                query.select(count_star()).get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "entries".to_string(),
        })?;

        Ok(count)
    }
}

#[async_trait]
impl EntryRepoable for EntryRepo {
    async fn list(&self, vault: &Vault, params: &ListEntriesParams) -> Result<PaginatedDto<Entry>> {
        let errors = params.validate();
        ensure!(
            errors.is_ok(),
            ValidationSnafu {
                msg: flatten_errors(&errors.unwrap_err()),
            }
        );

        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let vid = vault.id.clone();

        let total_records = self.listing_count(&vault.id, params).await?;
        let mut page: i32 = 1;
        let mut per_page: i32 = MAX_PER_PAGE;
        let mut offset: i64 = 0;

        if let Some(per_page_param) = params.per_page {
            if per_page_param > 0 && per_page_param <= MAX_PER_PAGE {
                per_page = per_page_param;
            }
        }

        let total_pages: i64 = (total_records as f64 / per_page as f64).ceil() as i64;

        if let Some(p) = params.page {
            let p64 = p as i64;
            if p64 > 0 && p64 <= total_pages {
                page = p;
                offset = (p64 - 1) * per_page as i64;
            }
        }

        // Do not query if we already know there are no records
        if total_pages == 0 {
            return Ok(PaginatedDto::new(Vec::new(), page, per_page, total_records));
        }

        let params_copy = params.clone();
        let select_res = db
            .interact(move |conn| {
                let mut query = dsl::entries.into_boxed();
                query = query.filter(dsl::vault_id.eq(vid.as_str()));

                if let Some(keyword) = params_copy.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(dsl::label.like(pattern));
                    }
                }
                query
                    .limit(per_page as i64)
                    .offset(offset)
                    .select(Entry::as_select())
                    .order(dsl::created_at.desc())
                    .load::<Entry>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "entries".to_string(),
        })?;

        Ok(PaginatedDto::new(items, page, per_page, total_records))
    }

    async fn create(&self, entry_dto: EntryDto) -> Result<Entry> {
        let file_db_pool = self.db_pool.clone();
        let db = file_db_pool.get().await.context(DbPoolSnafu)?;

        let entry: Entry = entry_dto.clone().into();
        let entry_copy = entry.clone();

        let insert_res = db
            .interact(move |conn| {
                diesel::insert_into(entries::table)
                    .values(&entry_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = insert_res.context(DbQuerySnafu {
            table: "entries".to_string(),
        })?;

        Ok(entry)
    }

    async fn get(&self, id: &str) -> Result<Option<Entry>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let fid = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::entries
                    .find(fid)
                    .select(Entry::as_select())
                    .first::<Entry>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let item = select_res.context(DbQuerySnafu {
            table: "entries".to_string(),
        })?;

        Ok(item)
    }

    async fn count_by_vault(&self, vault_id: &str) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let vid = vault_id.to_string();
        let count_res = db
            .interact(move |conn| {
                dsl::entries
                    .filter(dsl::vault_id.eq(vid.as_str()))
                    .select(count_star())
                    .get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "entries".to_string(),
        })?;

        Ok(count)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let eid = id.to_string();
        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::entries.filter(dsl::id.eq(eid))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "entries".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(feature = "test")]
pub struct EntryTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl EntryRepoable for EntryTestRepo {
    async fn list(
        &self,
        _vault: &Vault,
        _params: &ListEntriesParams,
    ) -> Result<PaginatedDto<Entry>> {
        Ok(PaginatedDto::new(vec![], 1, 10, 0))
    }

    async fn create(&self, _entry_dto: EntryDto) -> Result<Entry> {
        Err("Not supported".into())
    }

    async fn get(&self, _id: &str) -> Result<Option<Entry>> {
        Ok(None)
    }

    async fn count_by_vault(&self, _dir_id: &str) -> Result<i64> {
        Ok(0)
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
