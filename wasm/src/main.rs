use leptos::mount::mount_to_body;

mod app;
mod container;
mod error;

use app::App;

// Re-export error types for convenience
pub use error::{Error, Result};

fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(App);
}
