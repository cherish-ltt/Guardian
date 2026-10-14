# 扩展开发

> 本文档详解如何新增业务模块、使用内置工具与预置依赖，项目概览见 [README](../README.md)。

## 新模块建议怎么扩展

如果你要新增一个真实业务模块，建议按下面的顺序：

1. 先在 `domain/models` 定义领域模型
2. 在 `domain/services` 定义用例接口（仓储接口属于外圈，不放这里）
3. 在 `entities` 新增实体，并在 `migrations/` 新增 sqlx 迁移文件（`0002_xxx.sql` 的 `CREATE TABLE`）；
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

## 预置备用依赖

为了让新项目开箱更快，`Cargo.toml` 里额外预置了几类常用依赖，并且加了中文分类注释：

- `argon2`：密码哈希（正式依赖，开箱即用）
- `rayon`：CPU 密集型并行计算备用
- `reqwest`：对外 HTTP 调用
- `validator`：请求参数校验
- `regex`、`base64`、`once_cell`、`rand`、`derive_more`、`itertools`：常用工具库

这些库暂时不一定全部在业务里直接使用，因此**不在 `default` feature 里**，
需要时手动开启：

```bash
cargo build --features preset        # rayon / validator / regex / base64 / once_cell / rand / derive_more / itertools
cargo build --features http-client   # reqwest
cargo build --features preset,http-client
```

`Cargo.toml` 中的版本仍然逐个锁定，开启后即可直接 `use`，无需再改依赖声明。
