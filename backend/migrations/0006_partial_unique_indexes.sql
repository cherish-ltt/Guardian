-- 把「软删除 + 全表唯一索引」改成「只对未删除行唯一」。
--
-- 0002 建的唯一索引对已软删的行同样生效，导致删除后无法重建同名记录：
-- 新建时查重 SQL 都带 `deleted = 0`（查不到冲突），插入却被数据库拦下并返回 409。
-- 改成部分索引后，唯一性只约束未删除的行，删除后可以立即重建同名记录。
--
-- 迁移是追加式的：不改 0002 的建表语句，否则已应用过 0002 的库会因 checksum 不匹配拒绝启动。
-- 现有数据必然满足新的约束（旧索引比新索引更严格），因此重建不会失败。

DROP INDEX IF EXISTS uk_sys_user_username;
CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_user_username ON sys_user (username) WHERE deleted = 0;

DROP INDEX IF EXISTS uk_sys_role_code;
CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_role_code ON sys_role (role_code) WHERE deleted = 0;

DROP INDEX IF EXISTS uk_sys_post_code;
CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_post_code ON sys_post (post_code) WHERE deleted = 0;
