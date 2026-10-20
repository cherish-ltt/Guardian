-- 代码生成产物从 Spring/Java 改为本项目的 axum + sea-orm 分层：
-- 字段配置的类型列换成 Rust 类型，字段名与业务名改成 snake_case，
-- 并新增 `nullable` 记下数据库列的可空性（生成物用它决定是否包 `Option<T>`）。
ALTER TABLE gen_table_column RENAME COLUMN java_field TO rust_field;
ALTER TABLE gen_table_column RENAME COLUMN java_type TO rust_type;
ALTER TABLE gen_table_column ADD COLUMN nullable INTEGER NOT NULL DEFAULT 1;

-- 旧值是 Java 类型名；只映射已知的四个，其余保持原样。
UPDATE gen_table_column SET rust_type = CASE rust_type
    WHEN 'Long' THEN 'i64'
    WHEN 'Integer' THEN 'i32'
    WHEN 'BigDecimal' THEN 'f64'
    WHEN 'LocalDateTime' THEN 'String'
    ELSE rust_type
END;

-- 字段名由列名重算（列名本身就是 snake_case），业务名由表名去掉模块前缀重算。
UPDATE gen_table_column SET rust_field = lower(column_name);
UPDATE gen_table SET business_name = lower(
    CASE WHEN instr(table_name, '_') > 0
        THEN substr(table_name, instr(table_name, '_') + 1)
        ELSE table_name
    END
);

-- 可空性按当前库的真实表结构回填；表已删除或列已改名时保持默认值（可空）。
UPDATE gen_table_column SET nullable = COALESCE((
    SELECT CASE WHEN p."notnull" = 1 THEN 0 ELSE 1 END
    FROM gen_table g JOIN pragma_table_info(g.table_name) p
    WHERE g.id = gen_table_column.table_id
      AND p.name = gen_table_column.column_name
), 1);
