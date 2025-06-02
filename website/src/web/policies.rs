use std::result::Result as StdResult;

use crate::{Error, Result};
use dto::actor::Actor;
use dto::role::Permission;

pub enum Resource {
    Org,
    User,
    Vault,
    Entry,
}

pub enum Action {
    Create,
    Read,
    Update,
    Delete,
}

pub fn enforce_policy(actor: &Actor, resource: Resource, action: Action) -> Result<()> {
    let result = match resource {
        Resource::Org => enforce_org_permissions(actor, action),
        Resource::Vault => enforce_vaults_permissions(actor, action),
        Resource::User => enforce_users_permissions(actor, action),
        Resource::Entry => enforce_entry_permissions(actor, action),
    };

    match result {
        Ok(_) => Ok(()),
        Err(message) => Err(Error::Forbidden {
            msg: message.to_string(),
        }),
    }
}

fn enforce_entry_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::EntriesCreate],
            "You do not have permission to create entries.",
        ),
        Action::Read => (
            vec![Permission::EntriesList, Permission::EntriesView],
            "You do not have permission to view entries.",
        ),
        Action::Update => (
            vec![Permission::EntriesEdit],
            "You do not have permission to edit entries.",
        ),
        Action::Delete => (
            vec![Permission::EntriesDelete],
            "You do not have permission to delete entries.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}

fn enforce_org_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::OrgsCreate],
            "You do not have permission to create new orgs.",
        ),
        Action::Read => (
            vec![Permission::OrgsList, Permission::OrgsView],
            "You do not have permission to view orgs.",
        ),
        Action::Update => (
            vec![Permission::OrgsEdit],
            "You do not have permission to edit orgs.",
        ),
        Action::Delete => (
            vec![Permission::OrgsDelete],
            "You do not have permission to delete orgs.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}

fn enforce_vaults_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::VaultsCreate],
            "You do not have permission to create new vaults.",
        ),
        Action::Read => (
            vec![Permission::VaultsList, Permission::VaultsView],
            "You do not have permission to view vaults.",
        ),
        Action::Update => (
            vec![Permission::VaultsEdit],
            "You do not have permission to edit vaults.",
        ),
        Action::Delete => (
            vec![Permission::VaultsDelete],
            "You do not have permission to delete vaults.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}

fn enforce_users_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::UsersCreate],
            "You do not have permission to create new users.",
        ),
        Action::Read => (
            vec![Permission::UsersList, Permission::UsersView],
            "You do not have permission to view users.",
        ),
        Action::Update => (
            vec![Permission::UsersEdit],
            "You do not have permission to edit users.",
        ),
        Action::Delete => (
            vec![Permission::UsersDelete],
            "You do not have permission to delete users.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}
