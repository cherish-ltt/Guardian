-- 同一字典类型下只允许一个默认项。
--
-- 旧实现里 `is_default` 只是普通标记：前端表单可以同时勾选多项，库上也没有任何约束，
-- 线上已经存在同类型多个默认值的数据。迁移是追加式的：不改 0002 的建表语句，
-- 否则已应用过 0002 的库会因 checksum 不匹配拒绝启动。
--
-- 第一步按「排序 + 主键」保留每类型的第一项，其余降级为普通项：用户不需要做任何操作，
-- 下次读到该字典时看到的就是唯一默认值。第二步建部分唯一索引，从数据层挡住第二个默认项。

UPDATE sys_dict_data SET is_default = 0
WHERE is_default = 1
  AND EXISTS (
    SELECT 1 FROM sys_dict_data d
    WHERE d.dict_type = sys_dict_data.dict_type
      AND d.is_default = 1
      AND (d.sort, d.id) < (sys_dict_data.sort, sys_dict_data.id)
  );

CREATE UNIQUE INDEX IF NOT EXISTS uk_sys_dict_data_default
    ON sys_dict_data (dict_type) WHERE is_default = 1;
