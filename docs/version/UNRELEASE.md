# 未发布变更

本文件累积下一次发布要带上的变更，发版时并入 `vX.Y.Z.md` 后清空（规则见仓库根 `AGENTS.md`）。
小节结构与正式发布说明一致，按「后端」「前端」分别罗列，没有改动的部分保留小节标题并写「无」。

## 后端

- **生成器模板整体换成 Rust**：`src/services/generator.rs` 的产物从 4 个 Java 类
  （`domain` / `mapper` / `service` / `controller`）改为本项目的真实分层：
  `entities/<表名>.rs`（sea-orm `DeriveEntityModel`）、`domain/models/<表名>.rs`（列表行与写入命令）、
  `domain/services/<表名>.rs`（`XxxUseCase`）、`infrastructure/repositories/<表名>.rs`
  （仓储接口与 `SeaOrmXxxRepository`）、`services/<表名>.rs`（用例实现）、
  `api/controllers/<表名>.rs`（axum 路由与处理器），加上前端 api / 页面、菜单权限 SQL，
  以及一份 `docs/generated/<表名>.md` 接入说明，共 10 个文件。

  控制器按本项目后台管理的写法生成：`AdminUser` 提取器 + `require("模块:业务:动作")` 权限码、
  `ApiJson<T>` 请求体、`ok` / `done` 响应。服务层的列表查询已接好，新增 / 修改 / 删除留 `todo!`，
  仓储只给 trait 与适配器声明——缺失实现显式暴露，不静默返回伪成功。

- **字段配置改名为 Rust 语义**：`gen_table_column` 的 `java_field` / `java_type` 改名为
  `rust_field` / `rust_type`，类型值从 Java 类型名（`Long` / `BigDecimal` / `LocalDateTime`）
  换成 Rust 类型（`i64` / `i32` / `f64` / `String`）。接口字段随之变成 `rustField` / `rustType`。
  新增迁移 `0007_generator_rust_columns.sql` 转换存量行：重命名列、映射类型值、按列名重算
  `rust_field`、按表名重算 `business_name`。

- **业务名改用 snake_case**：`sys_user` 的业务名从 `userRole` 这类 camelCase 改为 `user`，
  与 Rust 模块名、函数名一致。

- **可空列生成 `Option<T>`**：`gen_table_column` 新增 `nullable` 列（迁移按当前库结构回填），
  生成物据此把可空列包成 `Option<T>`，与数据库的 NULL 语义一致。此前字段配置只记了
  `required`，它同时受主键与业务判断影响，无法替代可空性。

- **前端产物路径对齐项目现有布局**：api 产物平铺为 `frontend/src/api/<业务名>.ts`，
  页面产物为 `frontend/src/views/<模块>/<类名>.vue`（此前是 `api/<模块>/<业务名>.ts` 与
  `views/<模块>/<业务名>/index.vue`），菜单 SQL 的 `component` 指向 `<模块>/<类名>`。

- **预览默认展开改为按路径匹配**：`GenPreview.activeFile` 现在返回默认展开文件的完整路径
  （`backend/src/api/controllers/<表名>.rs`），与 `files[].path` 取值一致；此前返回文件名。
  ZIP 名仍取实体类名（`SysPost.zip`）。

- **控制器与项目现有写法一致**：只注册 `get` / `put`，用 `.route(..., put(update).delete(remove))`
  单条路由声明；服务层未实现的写方法参数加下划线前缀，避免生成物本身带未使用变量告警。

- **契约文档同步**：`backend/AGENTS.md` 10.10 的「7 个模板与 Spring 文本块逐字一致」改成
  「10 个模板产出本项目 axum + sea-orm 分层骨架，写操作留 `todo!`」；`backend/docs/ADMIN.md`
  的接口表与「与 Spring 版的差异」补入模板、字段命名、业务名、默认展开、可空列、
  前端产物与写操作七条。

## 前端

- **代码生成页的字段映射图改标 Rust**：三列标题从「数据库字段 / Java 属性 / 页面呈现」改为
  「数据库字段 / Rust 字段 / 页面呈现」，节点读取 `rustField` / `rustType`。
- **预览文件定位改为按路径**：`GeneratorPipeline.vue` 用 `preview.activeFile`（完整路径）匹配
  当前文件，配合后端 `activeFile` 语义变化。
- 页面描述改为「本项目 axum 分层的后端骨架、前端页面、菜单权限 SQL 与接入说明」。
