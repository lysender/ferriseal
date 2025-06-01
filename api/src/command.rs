use snafu::ResultExt;
use text_io::read;

use crate::Result;
use crate::config::Config;
use crate::error::PasswordPromptSnafu;
use crate::state::create_app_state;
use db::org::NewOrg;
use db::user::NewUser;

use crate::org::create_org;

pub async fn run_setup(config: &Config) -> Result<()> {
    print!("Enter username for the admin user: ");
    let username: String = read!("{}\n");

    let password = rpassword::prompt_password("Enter password for the admin user: ").context(
        PasswordPromptSnafu {
            msg: "Failed to read password",
        },
    )?;

    let password = password.trim().to_string();
    let new_user = NewUser {
        username: username.trim().to_string(),
        password,
        roles: "SystemAdmin".to_string(),
    };

    let state = create_app_state(config).await?;

    let org_id: String;
    let admin_org = state.db.org.find_admin().await?;
    if let Some(org) = admin_org {
        org_id = org.id;
    } else {
        let new_org = NewOrg {
            name: "system-admin".to_string(),
        };
        let org = create_org(&state, &new_org, true).await?;
        println!("{{ id = {}, name = {} }}", org.id, org.name);
        println!("Created system admin client.");
        org_id = org.id;
    }

    let users = state.db.users.list(&org_id).await?;
    if users.len() > 0 {
        println!("Admin user already exists.");
        return Ok(());
    }

    let user = state.db.users.create(&org_id, &new_user, true).await?;
    println!(
        "{{ id = {}, username = {} status = {} }}",
        user.id, user.username, user.status
    );
    println!("Created system admin user.");
    Ok(())
}
