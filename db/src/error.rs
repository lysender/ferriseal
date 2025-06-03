use deadpool_diesel::{InteractError, PoolError};
use dto::role::InvalidRolesError;
use snafu::{Backtrace, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Error getting db connection: {}", source))]
    DbPool {
        source: PoolError,
        backtrace: Backtrace,
    },

    #[snafu(display("Error using the db connection: {}", source))]
    DbInteract {
        source: InteractError,
        backtrace: Backtrace,
    },

    #[snafu(display("Error querying {}: {}", table, source))]
    DbQuery {
        table: String,
        source: diesel::result::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("{}", msg))]
    Validation { msg: String },

    #[snafu(display("{}", source))]
    Password {
        source: password::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("{}", source))]
    InvalidRoles {
        source: InvalidRolesError,
        backtrace: Backtrace,
    },

    #[snafu(display("{}", msg))]
    Whatever { msg: String },
}

// Allow string slices to be converted to Error
impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::Whatever {
            msg: val.to_string(),
        }
    }
}

impl From<String> for Error {
    fn from(val: String) -> Self {
        Self::Whatever { msg: val }
    }
}
