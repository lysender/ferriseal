CREATE TABLE entries (
    id CHAR(32) PRIMARY KEY NOT NULL,
    vault_id CHAR(32) NOT NULL,
    label VARCHAR(250) NOT NULL,
    cipher_username TEXT NULL,
    cipher_password TEXT NULL,
    cipher_notes TEXT NULL,
    cipher_extra_notes TEXT NULL,
    status VARCHAR(10) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY (vault_id) REFERENCES vaults(id)
);
CREATE INDEX entries_vault_id_idx ON entries(vault_id);
CREATE INDEX entries_vault_id_label_idx ON entries(vault_id, label);
CREATE INDEX entries_vault_id_created_at_idx ON entries(vault_id, created_at);
CREATE INDEX entries_vault_id_updated_at_idx ON entries(vault_id, updated_at);
