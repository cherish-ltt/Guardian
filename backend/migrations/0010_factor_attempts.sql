-- 2FA 阶段使用独立的失败计数。
--
-- `failures` / `blocked_until` 原本由口令阶段（password_guard）与 2FA 阶段共用：
-- 口令校验成功会把 `failures` 清零，而 2FA 阶段挨猜的成本远低于口令
-- （6 位数字，每次口令正确即可再试），共用计数等于给 2FA 开了无限重试。
-- 迁移是追加式的：不改 0003 的建表语句，已应用过 0003 的库会因 checksum 不匹配拒绝启动。

ALTER TABLE sys_user_security ADD COLUMN factor_failures INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_user_security ADD COLUMN factor_blocked_until INTEGER NOT NULL DEFAULT 0;
