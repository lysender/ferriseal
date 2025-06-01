CREATE TABLE vaults (
    id CHAR(32) PRIMARY KEY NOT NULL,
    org_id CHAR(32) NOT NULL,
    name VARCHAR(50) NOT NULL,
    test_cipher VARCHAR(255) NOT NULL,
    created_at BIGINT NOT NULL,
    FOREIGN KEY (org_id) REFERENCES orgs(id)
);

CREATE INDEX vaults_org_id_idx ON vaults(org_id);
