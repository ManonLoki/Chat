-- Add migration script here


-- Create Workspaces for users
CREATE TABLE workspaces (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(32) NOT NULL UNIQUE,
    owner_id BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

--Alter users table to add workspace_id
ALTER TABLE users
ADD COLUMN ws_id BIGINT REFERENCES workspaces(id);

--Alter chats table to add ws_id
ALTER TABLE chats
ADD COLUMN ws_id BIGINT REFERENCES workspaces(id);

BEGIN;
-- create fake super user
INSERT INTO users(id,fullname,email,password_hash)
    VALUES(0,'super user','fake_super@none.org','none');
INSERT INTO workspaces(id,name,owner_id)
    VALUES(0,'fake_workspace',0);
UPDATE users SET ws_id = 0 WHERE id = 0;
COMMIT;

-- ALTER users table workspace_id to not null
ALTER TABLE users ALTER COLUMN ws_id SET NOT NULL;
