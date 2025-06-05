use leptos::prelude::*;

fn main() {
    println!("well, hello...");
    leptos::mount::mount_to_body(|| view! { <p>"Hello, world!"</p> })
}
