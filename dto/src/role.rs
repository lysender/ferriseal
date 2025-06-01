use snafu::{Snafu, ensure};
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    SystemAdmin,
    Admin,
    Editor,
    Viewer,
}

#[derive(Debug, Snafu)]
#[snafu(display("Invalid roles: {roles}"))]
pub struct InvalidRolesError {
    roles: String,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    OrgsCreate,
    OrgsEdit,
    OrgsDelete,
    OrgsList,
    OrgsView,
    OrgsManage,

    VaultsCreate,
    VaultsEdit,
    VaultsDelete,
    VaultsList,
    VaultsView,
    VaultsManage,

    UsersCreate,
    UsersEdit,
    UsersDelete,
    UsersList,
    UsersView,
    UsersManage,

    EntriesCreate,
    EntriesEdit,
    EntriesDelete,
    EntriesList,
    EntriesView,
    EntriesManage,
}

#[derive(Debug, Snafu)]
#[snafu(display("Invalid permissions: {permissions}"))]
pub struct InvalidPermissionsError {
    permissions: String,
}

impl TryFrom<&str> for Role {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "SystemAdmin" => Ok(Role::SystemAdmin),
            "Admin" => Ok(Role::Admin),
            "Editor" => Ok(Role::Editor),
            "Viewer" => Ok(Role::Viewer),
            _ => Err(format!("Invalid role: {value}")),
        }
    }
}

impl core::fmt::Display for Role {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Role::SystemAdmin => write!(f, "SystemAdmin"),
            Role::Admin => write!(f, "Admin"),
            Role::Editor => write!(f, "Editor"),
            Role::Viewer => write!(f, "Viewer"),
        }
    }
}

pub fn to_roles(list: Vec<String>) -> Result<Vec<Role>, InvalidRolesError> {
    let mut roles: Vec<Role> = Vec::with_capacity(list.len());
    let mut errors: Vec<String> = Vec::with_capacity(list.len());
    for item in list.into_iter() {
        let role = item.as_str();
        match Role::try_from(role) {
            Ok(role) => roles.push(role),
            Err(_) => errors.push(role.to_string()),
        }
    }

    ensure!(
        errors.len() == 0,
        InvalidRolesSnafu {
            roles: errors.join(", ")
        }
    );

    Ok(roles)
}

impl TryFrom<&str> for Permission {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "orgs.create" => Ok(Permission::OrgsCreate),
            "orgs.edit" => Ok(Permission::OrgsEdit),
            "orgs.delete" => Ok(Permission::OrgsDelete),
            "orgs.list" => Ok(Permission::OrgsList),
            "orgs.view" => Ok(Permission::OrgsView),
            "orgs.manage" => Ok(Permission::OrgsManage),
            "users.create" => Ok(Permission::UsersCreate),
            "users.edit" => Ok(Permission::UsersEdit),
            "users.delete" => Ok(Permission::UsersDelete),
            "users.list" => Ok(Permission::UsersList),
            "users.view" => Ok(Permission::UsersView),
            "users.manage" => Ok(Permission::UsersManage),
            "vaults.create" => Ok(Permission::VaultsCreate),
            "vaults.edit" => Ok(Permission::VaultsEdit),
            "vaults.delete" => Ok(Permission::VaultsDelete),
            "vaults.list" => Ok(Permission::VaultsList),
            "vaults.view" => Ok(Permission::VaultsView),
            "vaults.manage" => Ok(Permission::VaultsManage),
            "entries.create" => Ok(Permission::EntriesCreate),
            "entries.edit" => Ok(Permission::EntriesEdit),
            "entries.delete" => Ok(Permission::EntriesDelete),
            "entries.list" => Ok(Permission::EntriesList),
            "entries.view" => Ok(Permission::EntriesView),
            "entries.manage" => Ok(Permission::EntriesManage),
            _ => Err(format!("Invalid permission: {value}")),
        }
    }
}

impl core::fmt::Display for Permission {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Permission::OrgsCreate => write!(f, "orgs.create"),
            Permission::OrgsEdit => write!(f, "orgs.edit"),
            Permission::OrgsDelete => write!(f, "orgs.delete"),
            Permission::OrgsList => write!(f, "orgs.list"),
            Permission::OrgsView => write!(f, "orgs.view"),
            Permission::OrgsManage => write!(f, "orgs.manage"),
            Permission::UsersCreate => write!(f, "users.create"),
            Permission::UsersEdit => write!(f, "users.edit"),
            Permission::UsersDelete => write!(f, "users.delete"),
            Permission::UsersList => write!(f, "users.list"),
            Permission::UsersView => write!(f, "users.view"),
            Permission::UsersManage => write!(f, "users.manage"),
            Permission::VaultsCreate => write!(f, "vaults.create"),
            Permission::VaultsEdit => write!(f, "vaults.edit"),
            Permission::VaultsDelete => write!(f, "vaults.delete"),
            Permission::VaultsList => write!(f, "vaults.list"),
            Permission::VaultsView => write!(f, "vaults.view"),
            Permission::VaultsManage => write!(f, "vaults.manage"),
            Permission::EntriesCreate => write!(f, "entries.create"),
            Permission::EntriesEdit => write!(f, "entries.edit"),
            Permission::EntriesDelete => write!(f, "entries.delete"),
            Permission::EntriesList => write!(f, "entries.list"),
            Permission::EntriesView => write!(f, "entries.view"),
            Permission::EntriesManage => write!(f, "entries.manage"),
        }
    }
}

pub fn to_permissions(
    permissions: &Vec<String>,
) -> Result<Vec<Permission>, InvalidPermissionsError> {
    let mut perms: Vec<Permission> = Vec::with_capacity(permissions.len());
    let mut errors: Vec<String> = Vec::with_capacity(permissions.len());
    for item in permissions.iter() {
        let perm = item.as_str();
        match Permission::try_from(perm) {
            Ok(permission) => perms.push(permission),
            Err(_) => errors.push(perm.to_string()),
        }
    }

    ensure!(
        errors.len() == 0,
        InvalidPermissionsSnafu {
            permissions: errors.join(", ")
        }
    );

    Ok(perms)
}

/// Role to permissions mapping
pub fn role_permissions(role: &Role) -> Vec<Permission> {
    match role {
        Role::SystemAdmin => vec![
            Permission::OrgsCreate,
            Permission::OrgsEdit,
            Permission::OrgsDelete,
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::OrgsManage,
            Permission::UsersCreate,
            Permission::UsersEdit,
            Permission::UsersDelete,
            Permission::UsersList,
            Permission::UsersView,
            Permission::UsersManage,
            Permission::VaultsCreate,
            Permission::VaultsEdit,
            Permission::VaultsDelete,
            Permission::VaultsList,
            Permission::VaultsView,
            Permission::VaultsManage,
        ],
        Role::Admin => vec![
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::VaultsList,
            Permission::VaultsView,
            Permission::UsersList,
            Permission::UsersView,
            Permission::EntriesCreate,
            Permission::EntriesEdit,
            Permission::EntriesDelete,
            Permission::EntriesList,
            Permission::EntriesView,
            Permission::EntriesManage,
        ],
        Role::Editor => vec![
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::VaultsList,
            Permission::VaultsView,
            Permission::EntriesCreate,
            Permission::EntriesList,
            Permission::EntriesView,
        ],
        Role::Viewer => vec![
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::VaultsList,
            Permission::VaultsView,
            Permission::EntriesList,
            Permission::EntriesView,
        ],
    }
}

/// Get all permissions for the given roles
pub fn roles_permissions(roles: &Vec<Role>) -> Vec<Permission> {
    let mut permissions: HashSet<Permission> = HashSet::new();
    roles.iter().for_each(|role| {
        role_permissions(role).iter().for_each(|p| {
            permissions.insert(p.clone());
        });
    });
    permissions.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_roles_valid() {
        let data = vec!["Admin".to_string(), "Viewer".to_string()];
        let roles = to_roles(data).unwrap();
        assert_eq!(roles, vec![Role::Admin, Role::Viewer]);
    }

    #[test]
    fn test_to_roles_invalid() {
        let data = vec![
            "Admin".to_string(),
            "InvalidRole".to_string(),
            "NetflixRole".to_string(),
        ];
        let roles = to_roles(data);
        assert!(roles.is_err());
        if let Err(e) = roles {
            assert_eq!(e.to_string(), "Invalid roles: InvalidRole, NetflixRole");
        }
    }

    #[test]
    fn test_to_permissions_valid() {
        let data = vec![
            "orgs.create".to_string(),
            "orgs.edit".to_string(),
            "orgs.delete".to_string(),
        ];
        let permissions = to_permissions(&data).unwrap();
        assert_eq!(
            permissions,
            vec![
                Permission::OrgsCreate,
                Permission::OrgsEdit,
                Permission::OrgsDelete,
            ]
        );
    }

    #[test]
    fn test_to_permissions_invalid() {
        let data = vec![
            "orgs.create".to_string(),
            "orgs.edit".to_string(),
            "orgs.delete".to_string(),
            "netflix.binge".to_string(),
            "netflix.watch".to_string(),
        ];
        let permissions = to_permissions(&data);
        assert!(permissions.is_err());
        if let Err(e) = permissions {
            assert_eq!(
                e.to_string(),
                "Invalid permissions: netflix.binge, netflix.watch"
            );
        }
    }
}
