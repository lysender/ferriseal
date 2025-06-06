pub mod entries;
pub mod error;
pub mod index;
pub mod login;
pub mod logout;
pub mod middleware;
pub mod my_vault;
pub mod orgs;
pub mod policies;
pub mod pref;
pub mod profile;
pub mod routes;
pub mod users;
pub mod vaults;

pub const AUTH_TOKEN_COOKIE: &str = "auth_token";
pub const THEME_COOKIE: &str = "theme";

pub use error::*;
pub use index::*;
pub use login::*;
pub use logout::*;
pub use policies::*;
pub use pref::*;
pub use routes::*;
