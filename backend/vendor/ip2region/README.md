# vendor/ip2region

`ip2region` 官方 Rust 查询客户端的副本，用于把客户端 IP 解析成离线地名（登录日志与操作日志的
`location` 列）。离线库文件不随仓库分发，部署时下载，见 `docs/DEPLOYMENT.md`。

- 来源：https://github.com/lionsoul2014/ip2region `binding/rust/ip2region`
- 锁定提交：`1bf6cabfb63c1a14af0971749c2120a3e5c77d72`（2026-10-14）
- 许可：`Apache-2.0 OR MIT`（上游双许可，见 [LICENSE.md](LICENSE.md)）

## 为什么是 vendor 而不是依赖

官方 crate 未发布到 crates.io，且它的 `maker` 依赖是**运行时依赖**：`maker` 是生成 xdb 的
命令行工具，会带入 `clap`、`chrono`、`itertools`、`tracing-subscriber`、`bytes`、`num-derive`
等约 70 个与本服务无关的包。这里保留官方查询实现，并把 `maker` 换成只含查询侧所需的
`header.rs`。

## 与上游的差异

| 文件 | 差异 |
| --- | --- |
| `src/header.rs` | 新增。取自上游 `maker/src/header.rs` 的常量与 `Header` 解析，去掉生成侧字段与 `IndexPolicy`、`IPAddrExt`，不依赖 `bytes` / `num-derive` / `clap` |
| `src/searcher.rs` | 引用 `crate::header` 取代 `maker`；删除上游的测试模块（它依赖仓库内 113 MB 的原始数据文件） |
| `src/error.rs` | `MakerError` 改指向 `crate::header` |
| `src/ip_value.rs` | 未改动 |

代码按本仓库的 `.rustfmt.toml` 格式化（与上游只有换行差异，不影响逻辑）。

查询逻辑与上游逐行一致，升级方式：按上述差异重新套用新版本的上游文件，并在本项目里跑一次

```bash
IP_LOCATION_DB_PATH=/path/to/ip2region_v4.xdb cargo test --features docs
```

加上一次真实登录，确认 `sys_login_log.location` 仍是地名。
