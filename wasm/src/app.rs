use leptos::prelude::*;
use reactive_stores::Store;

#[component]
pub fn App() -> impl IntoView {
    let (count, set_count) = signal(0);
    let double_count = move || count.get() * 2;

    let values = vec![0, 1, 2];

    view! {
        <button
            on:click=move |_| {
                *set_count.write() += 1;
            }
            class=(["is-danger"], move || count.get() % 2 == 1)
            class="button"
        >
            "Click me: "
            {count}
        </button>
        <br />
        <ProgressBar progress=count />
        <br />
        <ProgressBar progress=Signal::derive(double_count) />

        <div>
            <ul>{values.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}</ul>
        </div>

        <div>
            <ForWithEntries />
        </div>

        <SimpleForm />

        <NumericInput />
    }
}

#[derive(Store, Debug, Clone)]
pub struct Data {
    #[store(key: String = |row| row.key.clone())]
    rows: Vec<DatabaseEntry>,
}

#[derive(Store, Debug, Clone)]
struct DatabaseEntry {
    key: String,
    value: i32,
}

#[component]
fn ProgressBar(
    #[prop(default = 100)] max: u16,
    #[prop(into)] progress: Signal<i32>,
) -> impl IntoView {
    view! { <progress class="progress" max=max value=progress /> }
}

#[component]
fn ForWithEntries() -> impl IntoView {
    // start with a set of three rows
    let data = Store::new(Data {
        rows: vec![
            DatabaseEntry {
                key: "foo".to_string(),
                value: 10,
            },
            DatabaseEntry {
                key: "bar".to_string(),
                value: 20,
            },
            DatabaseEntry {
                key: "baz".to_string(),
                value: 15,
            },
        ],
    });

    view! {
        // when we click, update each row,
        // doubling its value
        <button on:click=move |_| {
            use reactive_stores::StoreFieldIterator;
            for row in data.rows().iter_unkeyed() {
                *row.value().write() *= 2;
            }
            leptos::logging::log!("{:?}", data.get());
        }>"Update Values"</button>
        // iterate over the rows and display each value
        <For
            each=move || data.rows()
            key=|row| row.read().key.clone()
            children=|child| {
                let value = child.value();
                view! { <p>{move || value.get()}</p> }
            }
        />
    }
}

#[component]
fn SimpleForm() -> impl IntoView {
    let (name, set_name) = signal("Controlled".to_string());
    let email = RwSignal::new("".to_string());
    let favorite_color = RwSignal::new("red".to_string());
    let spam_me = RwSignal::new(true);

    view! {
        <div>
            <input class="input" type="text" bind:value=(name, set_name) />
            <input class="input" type="email" bind:value=email />
            <label class="checkbox">
                "Please send me lots of spam email." <input type="checkbox" bind:checked=spam_me />
            </label>
            <fieldset class="control">
                <legend>"Favorite color"</legend>
                <label class="radio">
                    "Red" <input type="radio" name="color" value="red" bind:group=favorite_color />
                </label>
                <label class="radio">
                    "Green"
                    <input type="radio" name="color" value="green" bind:group=favorite_color />
                </label>
                <label class="radio">
                    "Blue"
                    <input type="radio" name="color" value="bluee" bind:group=favorite_color />
                </label>
            </fieldset>
            <p>"Your favorite color is " {favorite_color} "."</p>
            <p>"Name is: " {name}</p>
            <p>"Email is: " {email}</p>
            <Show when=move || spam_me.get()>
                <p>"You’ll receive cool bonus content!"</p>
            </Show>
        </div>
    }
}

#[component]
fn NumericInput() -> impl IntoView {
    let (value, set_value) = signal(Ok(0));

    view! {
        <h1>"Error Handling"</h1>

        <label>
            "Type a number (or something that's not a number)"
            <input
                type="number"
                on:input:target=move |ev| { set_value.set(ev.target().value().parse::<i32>()) }
            />
            <ErrorBoundary fallback=|errors| {
                view! {
                    <div class="error">
                        <p>"Not a number! Errors: "</p>
                        <ul>
                            {move || {
                                errors
                                    .get()
                                    .into_iter()
                                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                                    .collect::<Vec<_>>()
                            }}
                        </ul>
                    </div>
                }
            }>
                <p>"You entered " <strong>{value}</strong></p>
            </ErrorBoundary>
        </label>
    }
}
