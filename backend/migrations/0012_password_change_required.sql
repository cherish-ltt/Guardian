ALTER TABLE sys_user ADD COLUMN password_change_required INTEGER NOT NULL DEFAULT 0
    CHECK (password_change_required IN (0, 1));
