// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Text,
        username -> Text,
        password -> Text,
        status -> Text,
        roles -> Text,
        created_at -> BigInt,
        updated_at -> BigInt,
    }
}
