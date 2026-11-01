ALTER TABLE gen_table_column ADD COLUMN primary_key INTEGER NOT NULL DEFAULT 0;

UPDATE gen_table_column SET
    primary_key = COALESCE((SELECT CASE WHEN p.pk > 0 THEN 1 ELSE 0 END
        FROM gen_table g JOIN pragma_table_info(g.table_name) p
        WHERE g.id = gen_table_column.table_id AND p.name = gen_table_column.column_name), 0),
    nullable = COALESCE((SELECT CASE WHEN p.pk > 0 OR p."notnull" = 1 THEN 0 ELSE 1 END
        FROM gen_table g JOIN pragma_table_info(g.table_name) p
        WHERE g.id = gen_table_column.table_id AND p.name = gen_table_column.column_name), nullable);

UPDATE gen_table_column SET is_insert = 0, is_edit = 0, is_list = 0, required = 0
WHERE primary_key = 1;

UPDATE gen_table_column SET is_insert = 1, is_edit = 1, is_list = 1,
    required = CASE WHEN nullable = 0 THEN 1 ELSE 0 END
WHERE column_name = 'id' AND primary_key = 0;
