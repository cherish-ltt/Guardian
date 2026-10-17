ALTER TABLE sys_user_security ADD COLUMN login_challenge_hash TEXT;
ALTER TABLE sys_user_security ADD COLUMN login_challenge_expires INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_user_security ADD COLUMN login_password_hash TEXT;
CREATE UNIQUE INDEX idx_user_login_challenge ON sys_user_security(login_challenge_hash);
