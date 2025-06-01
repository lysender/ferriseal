CREATE TABLE users (
    id CHAR(32) PRIMARY KEY NOT NULL,
    org_id CHAR(32) NOT NULL,
    username VARCHAR(30) NOT NULL,
    password VARCHAR(250) NOT NULL,
    status VARCHAR(10) NOT NULL,
    roles VARCHAR(250) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY (org_id) REFERENCES orgs(id)
);
CREATE INDEX users_org_id_idx ON users(org_id);
CREATE UNIQUE INDEX users_username_idx ON users(username);
