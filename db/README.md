# vault-db: Database Crate

## Migration setup

```
diesel setup --database-url=sqlite://db/db.sqlite3
diesel migration generate create_orgs --database-url=sqlite://db/db.sqlite3
diesel migration generate create_users --database-url=sqlite://db/db.sqlite3
diesel migration generate create_vaults --database-url=sqlite://db/db.sqlite3
diesel migration generate create_entries --database-url=sqlite://db/db.sqlite3

diesel migration run --database-url=sqlite://db/db.sqlite3
diesel migration redo --database-url=sqlite://db/db.sqlite3
diesel migration revert --database-url=sqlite://db/db.sqlite3
```

## Orgs

Org:
- id
- name
- admin
- created_at

## Users

User:
- id
- org_id
- username
- password
- status: active, inactive
- roles: csv of roles
- created_at
- updated_at

## Vaults

Vault:
- id
- org_id
- name
- test_cipher
- created_at
- updated_at

## Vault Entries

Entry:
- id
- vault_id
- label
- username
- password
- notes
- extra_notes
- status
- created_at
- updated_at
