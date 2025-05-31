# vault-api: Password Manager

`vault-api` is a simple password manager service and is the backend service for `vault-website`.

This service is part of the low-cost hosting solutions initiative to help people reduce tech cost.

Objectives:
- Store passwords and secrets
- Encrypt data at the client side
- Do not store the decryption key
- Derive the decryption key from a vault password not stored in the system

Workflow:
- Assigne a name or label for the data entry
- User encrypts passwords or secrets
- Send the encrypted data to the API for processing
- Store the encrypted data in the database
- User search for the password entry
- Service returns the matching entries (id and label, etc)
- User selects an entry
- Service returns the full data (still encrypted)
- User unseals the data by entering the vault password
- User may not have to enter the vault password all the time
- Encrypted data decrypted in the client
- User should be able to copy the password or secret to clipboard

## Migration setup

```
diesel setup --database-url=sqlite://db/db.sqlite3
diesel migration generate create_users --database-url=sqlite://db/db.sqlite3
diesel migration generate create_vaults --database-url=sqlite://db/db.sqlite3
diesel migration generate create_entries --database-url=sqlite://db/db.sqlite3
```

## Encryption crates

- `aes-gcm`

It is designed for personal use and not indended for large number of concurrent users.
The goal of the service is to provide an economical way to store and retrieve
files in the cloud.

Uses cases:
- Store personal files and documents
- Online photo album

## Features 

- [x] JSON API endpoints to manage files
- [x] Multi-tenant
- [x] Multi-bucket
- [x] Google Cloud Storage
- [x] SQLite database
- [x] JWT authentication
- [x] Role based authorization
- [x] Tenants/clients management via CLI
- [x] Users management via CLI
- [x] Buckets management via CLI

## Workflow

- Tenants/clients are like organizations
- Each client have users and cloud storage buckets
- Files are organized in directories under a bucket
- Each directory have the following sub-directories:
  - orig - original file
  - preview - web optimized image preview
  - thumb - web optimized image thumbnail
- File metadata is collected
  - Content type
  - Size
  - Image dimension for each version
  - Date picture is taken

## Google Cloud Service Account

Create a Google Cloud Service Account with the following roles:
- Storage Folder Admin
- Storage HMAC Key Admin
- Storage Insights Collector Service
- Storage Object Admin

## Users

User:
- id
- username
- password
- status: active, inactive
- roles: csv of roles
- created_at
- updated_at

## Vaults

Vault:
- id
- name
- cipher_key
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
- extra_nodes
- status
- created_at
- updated_at

### Roles

- Admin
- Viewer

### Permissions

- vaults.create
- vaults.edit
- vaults.delete
- vaults.list
- vaults.view
- vaults.manage
- users.create
- users.edit
- users.delete
- users.list
- users.view
- users.manage
- entries.create
- entries.edit
- entries.delete
- entries.list
- entries.view
- entries.manage

### Roles to Permissions Mapping

Admin:
- vaults.create
- vaults.edit
- vaults.delete
- vaults.list
- vaults.view
- vaults.manage
- users.create
- users.edit
- users.delete
- users.list
- users.view
- users.manage
- entries.create
- entries.edit
- entries.delete
- entries.list
- entries.view
- entries.manage

Viewer:
- vaults.list
- vaults.view
- users.list
- users.view
- entries.list
- entries.view

## API Endpoints for regular users

```
GET /v1/auth/token
GET /v1/buckets
GET /v1/buckets/{bucket_id}
PATCH /v1/buckets/{bucket_id}
DELETE /v1/buckets/{bucket_id}
GET /v1/buckets/{bucket_id}/dirs?page=1&per_page=10&keyword=
POST /v1/buckets/{bucket_id}/dirs
GET /v1/buckets/{bucket_id}/dirs/{dir_id}
PATCH /v1/buckets/{bucket_id}/dirs/{dir_id}
DELETE /v1/buckets/{bucket_id}/dirs/{dir_id}
GET /v1/buckets/{bucket_id}/dirs/{dir_id}/files?page=1&per_page=10&keyword=
POST /v1/buckets/{bucket_id}/dirs/{dir_id}/files
GET /v1/buckets/{bucket_id}/dirs/{dir_id}/files/{file_id}
DELETE /v1/buckets/{bucket_id}/dirs/{dir_id}/files/{file_id}
```

## System Admin Endpoints

```
GET /v1/clients
POST /v1/clients
GET /v1/clients/{client_id}
PATCH /v1/clients/{client_id}
DELETE /v1/clients/{client_id}
GET /v1/clients/{client_id}/users
POST /v1/clients/{client_id}/users
GET /v1/clients/{client_id}/users/{user_id}
PATCH /v1/clients/{client_id}/users/{user_id}
DELETE /v1/clients/{client_id}/users/{user_id}
GET /v1/clients/{client_id}/buckets
POST /v1/clients/{client_id}/buckets
GET /v1/clients/{client_id}/buckets/{bucket_id}
PATCH /v1/clients/{client_id}/buckets/{bucket_id}
DELETE /v1/clients/{client_id}/buckets/{bucket_id}
```

## Database client setup

```
# Only when not yet installed 
sudo apt-get -y install libsqlite3-dev

# Required by our ORM and migration tool
cargo install diesel_cli --no-default-features --features sqlite
```

## Configuration by ENV variables

```
DATABASE_URL=sqlite://db/db.sqlite3
JWT_SECRET=value
UPLOAD_DIR=/path/to/upload_dir
PORT=11001

GOOGLE_PROJECT_ID=value
GOOGLE_APPLICATION_CREDENTIALS=/path/to/credentials.json
```

## Build binary

```
cargo build --release
```

## Deployment

You can deploy the application in many ways. In this example, we deploy
it as a simple systemd service.

### Setup systemd

File: `/data/scripts/memo-rs/run-api.sh`

```bash
#!/bin/sh
/data/www/html/sites/memo-rs/target/release/api -c /data/www/html/sites/memo-rs/api/config.toml server
```

Edit systemd service file:

```
sudo systemctl edit --force --full memo-api.service
```

File: `/etc/systemd/system/memo-api.service`

```
[Unit]
Description=memo-api Make memories

[Service]
User=www-data
Group=www-data

WorkingDirectory=/data/www/html/sites/memo-rs/api/
ExecStart=/data/scripts/memo-rs/run-api.sh
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

To enable it for the first time:

```
sudo systemctl enable memo-api.service
```

Various commands:

```
sudo systemctl start memo-api.service
sudo systemctl stop memo-api.service
sudo systemctl restart memo-api.service
sudo systemctl status memo-api.service
```
