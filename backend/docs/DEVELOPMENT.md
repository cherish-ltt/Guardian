# 扩展开发

> 本文档详解如何新增业务模块、使用内置工具与预置依赖，项目概览见 [README](../README.md)。

## 新模块建议怎么扩展

如果你要新增一个真实业务模块，建议按下面的顺序：

1. 先在 `domain/models` 定义领域模型
2. 在 `domain/services` 定义用例接口（仓储接口属于外圈，不放这里）
3. 在 `entities` 新增实体，并在 `migrations/` 新增 sqlx 迁移文件（`000N_xxx.sql` 的 `CREATE TABLE`）；
   迁移只能追加，不能改已发布的文件（改会让已应用它的库 checksum 不匹配而拒启）；
   老库的补列、补索引与幂等种子写在 `infrastructure/databases/schema.rs`
4. 在 `infrastructure/repositories` 写仓储接口与 SeaORM 适配器，方法接收 `C: ConnectionTrait`
5. 在 `services` 写用例实现，**需要多步写入时在 `services/transaction.rs` 里找事务写法**
6. 在 `container.rs` 注入实现
7. 最后在 `api/controllers` 和 `api/dto` 暴露 HTTP 接口，并补 `docs.rs` 的 OpenAPI 声明

这样做的好处是：

- 先定业务边界，再接具体框架
- 先写抽象，再接外部实现
- 未来替换数据库或拆微服务时，冲击面更小

## 新增工具

- `src/util/password.rs`
  - `hash_password`
  - `verify_password`
- 默认密码算法：`argon2`

示例：

```rust
use guardian_backend::util::password::{hash_password, verify_password};

let hashed = hash_password("S3cure-Password!")?;
let ok = verify_password("S3cure-Password!", &hashed)?;
```

## 新增依赖

`Cargo.toml` 不预置「备用依赖」：依赖必须当次就有调用点，否则不写进清单
（原先的 `preset` / `http-client` 两个 bundle 因零引用已整组删除）。需要新库时：

1. 按分类放到对应的 `#` 注释组下，版本用 `=` 精确锁定（见 AGENTS.md 第 3 节）；
2. 只开代码真正用到的 feature，不用 `full` 之类的聚合开关；依赖自带默认集时
   先用 `default-features = false` 关掉再显式开启；
3. 跑一遍完整检查：`cargo fmt --all -- --check`、`cargo clippy --all-targets --features docs -- -D warnings`、
   `cargo check --release --features docs`、`cargo test --features docs`。

依赖与 feature 的收紧规则（含 `axum` / `sea-orm` 关默认集、SQLite 不启用 TLS 等）
见 [AGENTS.md](../AGENTS.md) 第 10.12 节。
