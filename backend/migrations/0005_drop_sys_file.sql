-- 删除从未被使用的 sys_file 表。
--
-- 该表在 0002 里建好后没有任何 controller / service / repository 引用，也没有上传接口，
-- 留着只会让人以为文件模块存在。迁移文件是追加式的：不改 0002 的建表语句，
-- 否则已应用过 0002 的库会因 checksum 不匹配而拒绝启动。
DROP TABLE IF EXISTS sys_file;
