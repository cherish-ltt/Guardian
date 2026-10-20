# 未发布变更

本文件累积下一次发布要带上的变更，发版时并入 `vX.Y.Z.md` 后清空（规则见仓库根 `AGENTS.md`）。
小节结构与正式发布说明一致，按「后端」「前端」分别罗列，没有改动的部分保留小节标题并写「无」。

## 后端

- **生成物的仓储补齐实现**：`infrastructure/repositories/<表名>.rs` 现在生成
  `impl XxxRepository for SeaOrmXxxRepository`，`count_*` / `list_*` 按字段配置拼出可执行的
  SQL（关键词只匹配文本列），`row_of` 负责从查询结果构造列表行；新增 / 修改 / 删除仍留 `todo!`。
  此前只生成 trait 与空结构体，而服务层的列表查询直接调用这些方法，生成物放进项目必然
  `cargo check` 失败（`E0599: no method named count_xxx`）。

- **主键不再被当成可空列**：SQLite 的 `pragma_table_info` 对 `INTEGER PRIMARY KEY` 报
  `notnull=0`，导入时会把主键记成可空，生成物因此把主键写成 `Option<i64>`。导入时的字段推导
  与迁移 `0008` 都按「主键恒非空」处理。

- **迁移 `0008_gen_column_primary_not_null.sql`**：把已导入配置里的主键列 `nullable` 纠正为 0。
  `0007` 已随 `v0.6.0` 发布，不能修改其内容（sqlx 校验 checksum），修正只能以新迁移追加。

## 前端

- **代码预览的文件分组标题完整显示路径**：`GeneratorPipeline.vue` 左侧文件树里每组上方的
  灰色路径原来单行截断（`backend/src/infrastruc…`），改为自动换行，长路径能看全。
