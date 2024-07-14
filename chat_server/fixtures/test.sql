INSERT INTO workspaces(name,owner_id)
VALUES
('acme',0),
('foo',0),
('bar',0);

-- insert users
INSERT INTO users(ws_id,email,fullname,password_hash)
VALUES
(1, 'manonloki@gmail.com','manonloki','$argon2id$v=19$m=19456,t=2,p=1$q/d5qrN85MnoIpOLJashEw$9v2T4/DgIwcVE7TOOhhUZYWc7YLcWBTBjIP1yzXzTDU'),
(1, 'heather@163.com','heather','$argon2id$v=19$m=19456,t=2,p=1$q/d5qrN85MnoIpOLJashEw$9v2T4/DgIwcVE7TOOhhUZYWc7YLcWBTBjIP1yzXzTDU'),
(1, 'guest@gmail.com','guest','$argon2id$v=19$m=19456,t=2,p=1$q/d5qrN85MnoIpOLJashEw$9v2T4/DgIwcVE7TOOhhUZYWc7YLcWBTBjIP1yzXzTDU'),
(1, 'dingzd@gmail.com','dingzd','$argon2id$v=19$m=19456,t=2,p=1$q/d5qrN85MnoIpOLJashEw$9v2T4/DgIwcVE7TOOhhUZYWc7YLcWBTBjIP1yzXzTDU'),
(1, 'luhao@gmail.com','luhao','$argon2id$v=19$m=19456,t=2,p=1$q/d5qrN85MnoIpOLJashEw$9v2T4/DgIwcVE7TOOhhUZYWc7YLcWBTBjIP1yzXzTDU');

-- insert chats
-- insert public / private channel
INSERT INTO chats(ws_id,name,type,members)
VALUES
(1,'general','public_channel','{1,2,3,4,5}'),
(1,'private','private_channel','{1,2,3}');

-- insert unnamed chat
INSERT INTO chats(ws_id,type,members)
VALUES
(1, 'single', '{1,2}'),
(1, 'group', '{1,3,4}');




CREATE TABLE IF NOT EXISTS messages(
    id BIGSERIAL PRIMARY KEY,
    chat_id BIGINT NOT NULL  REFERENCES chats(id),
    sender_id BIGINT NOT NULL REFERENCES users(id),
    content TEXT NOT NULL,
    files TEXT[],
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
-- insert messages 10 messages for each chat
INSERT INTO messages(chat_id,sender_id,content)
VALUES
(1,1,'hello'),
(1,1,'hi'),
(1,1,'hey'),
(1,1,'yo'),
(1,1,'sup'),
(1,1,'oh'),
(1,1,'ha'),
(1,1,'you'),
(1,1,'nin'),
(1,1,'san');
