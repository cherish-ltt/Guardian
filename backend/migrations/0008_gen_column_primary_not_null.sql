-- 主键恒非空，但 SQLite 的 `pragma_table_info` 对 `INTEGER PRIMARY KEY` 报 notnull=0，
-- 0007 回填可空性时把主键记成了可空，生成物因此把主键写成 `Option<i64>`。
UPDATE gen_table_column SET nullable = 0 WHERE column_name = 'id';
