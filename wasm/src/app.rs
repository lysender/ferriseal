use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    let (count, set_count) = signal(0);
    let double_count = move || count.get() * 2;

    view! {
        <button
            on:click=move |_| {
                *set_count.write() += 1;
            }
            class:red=move || count.get() % 2 == 1
        >
            "Click me: "
            {count}
        </button>
        <br />
        <ProgressBar progress=count />
        <br />
        <ProgressBar progress=Signal::derive(double_count) />
    }
}

#[component]
fn ProgressBar(
    #[prop(default = 100)] max: u16,
    #[prop(into)] progress: Signal<i32>,
) -> impl IntoView {
    view! { <progress max=max value=progress /> }
}
