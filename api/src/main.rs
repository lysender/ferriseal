use clap::Parser;
use config::CliArgs;
use run::run_command;
use snafu::ErrorCompat;
use std::process;

mod auth;
mod command;
mod config;
mod entry;
mod entry;
mod error;
mod health;
mod org;
mod run;
mod schema;
mod state;
mod token;
mod user;
mod vault;
mod web;

// Re-export error types for convenience
pub use error::{Error, Result};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let args = CliArgs::parse();

    if let Err(e) = run_command(args).await {
        eprintln!("Application error: {}", e);
        if let Some(bt) = ErrorCompat::backtrace(&e) {
            println!("{}", bt);
        }
        process::exit(1);
    }
}
