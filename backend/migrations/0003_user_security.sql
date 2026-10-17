CREATE TABLE sys_user_security (
    user_id INTEGER PRIMARY KEY NOT NULL REFERENCES sys_user(id),
    secret TEXT,
    pending_secret TEXT,
    pending_expires INTEGER NOT NULL DEFAULT 0,
    last_step INTEGER NOT NULL DEFAULT -1,
    recovery_hashes TEXT NOT NULL DEFAULT '[]',
    auth_version INTEGER NOT NULL DEFAULT 0,
    failures INTEGER NOT NULL DEFAULT 0,
    blocked_until INTEGER NOT NULL DEFAULT 0
);

INSERT INTO sys_menu (id, parent_id, menu_name, menu_type, permission, sort, visible, status, create_by, update_by)
SELECT MAX(COALESCE(MAX(id), 0), 10000) + 1, 101, '重置2FA', 'btn', 'system:user:reset2fa', 5, 1, 1, 'system', 'system'
FROM sys_menu
HAVING NOT EXISTS (SELECT 1 FROM sys_menu WHERE permission = 'system:user:reset2fa');

INSERT OR IGNORE INTO sys_role_menu (role_id, menu_id)
SELECT DISTINCT r.id, m.id FROM sys_role r CROSS JOIN sys_menu m
WHERE m.permission = 'system:user:reset2fa' AND (
    r.role_code = 'super_admin' OR EXISTS (
        SELECT 1 FROM sys_role_menu rm JOIN sys_menu p ON p.id = rm.menu_id
        WHERE rm.role_id = r.id AND p.permission = 'system:user:resetPwd'
    )
);
