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
