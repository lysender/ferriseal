// @generated automatically by Diesel CLI.

diesel::table! {
    entries (id) {
        id -> Text,
        vault_id -> Text,
        label -> Text,
        cipher_username -> Nullable<Text>,
        cipher_password -> Nullable<Text>,
        cipher_notes -> Nullable<Text>,
        cipher_extra_notes -> Nullable<Text>,
        status -> Text,
        created_at -> BigInt,
        updated_at -> BigInt,
    }
}

diesel::table! {
    orgs (id) {
        id -> Text,
        name -> Text,
        created_at -> BigInt,
    }
}

diesel::table! {
    users (id) {
        id -> Text,
        org_id -> Text,
        username -> Text,
        password -> Text,
        status -> Text,
        roles -> Text,
        created_at -> BigInt,
        updated_at -> BigInt,
    }
}

diesel::table! {
    vaults (id) {
        id -> Text,
        org_id -> Text,
        name -> Text,
        test_cipher -> Text,
        created_at -> BigInt,
    }
}

diesel::joinable!(entries -> vaults (vault_id));
diesel::joinable!(users -> orgs (org_id));
diesel::joinable!(vaults -> orgs (org_id));

diesel::allow_tables_to_appear_in_same_query!(
    entries,
    orgs,
    users,
    vaults,
);
