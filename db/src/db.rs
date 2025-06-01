use std::sync::Arc;

use deadpool_diesel::sqlite::{Manager, Pool, Runtime};

use crate::{
    entry::{EntryRepo, EntryRepoable},
    org::{OrgRepo, OrgRepoable},
    user::{UserRepo, UserRepoable},
    vault::{VaultRepo, VaultRepoable},
};

pub fn create_db_pool(database_url: &str) -> Pool {
    let manager = Manager::new(database_url, Runtime::Tokio1);
    Pool::builder(manager).max_size(8).build().unwrap()
}

pub struct DbMapper {
    pub vaults: Arc<dyn VaultRepoable>,
    pub orgs: Arc<dyn OrgRepoable>,
    pub entries: Arc<dyn EntryRepoable>,
    pub users: Arc<dyn UserRepoable>,
}

pub fn create_db_mapper(database_url: &str) -> DbMapper {
    let pool = create_db_pool(database_url);
    DbMapper {
        vaults: Arc::new(VaultRepo::new(pool.clone())),
        orgs: Arc::new(OrgRepo::new(pool.clone())),
        entries: Arc::new(EntryRepo::new(pool.clone())),
        users: Arc::new(UserRepo::new(pool.clone())),
    }
}

#[cfg(feature = "test")]
pub fn create_test_db_mapper() -> DbMapper {
    use crate::entry::EntryTestRepo;
    use crate::org::OrgTestRepo;
    use crate::user::UserTestRepo;
    use crate::vault::VaultTestRepo;

    DbMapper {
        vaults: Arc::new(VaultTestRepo {}),
        orgs: Arc::new(OrgTestRepo {}),
        entries: Arc::new(EntryTestRepo {}),
        users: Arc::new(UserTestRepo {}),
    }
}
