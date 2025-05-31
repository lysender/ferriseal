CREATE TABLE users (
    id CHAR(32) PRIMARY KEY NOT NULL,
    username VARCHAR(30) NOT NULL,
    password VARCHAR(250) NOT NULL,
    status VARCHAR(10) NOT NULL,
    roles VARCHAR(250) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);
CREATE UNIQUE INDEX users_username_idx ON users(username);
