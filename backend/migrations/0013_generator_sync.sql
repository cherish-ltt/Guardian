ALTER TABLE gen_table ADD COLUMN version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gen_table_column ADD COLUMN column_type TEXT NOT NULL DEFAULT '';

UPDATE gen_table_column SET column_type = jdbc_type;
