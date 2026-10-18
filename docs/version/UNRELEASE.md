# 未发布变更

本文件累积下一次发布要带上的变更，发版时并入 `vX.Y.Z.md` 后清空（规则见仓库根 `AGENTS.md`）。
小节结构与正式发布说明一致，按「后端」「前端」分别罗列，没有改动的部分保留小节标题并写「无」。

## 后端

- **发布产物结构简化**：`release.yml` 打出的后端压缩包内，二进制由 `target/release/guardian-backend`
  改为根位置的 `guardian-backend`，与 `migrations/`、`.env-example` 三者并列，解压即可部署。
  打包步骤额外打印包内清单，发布日志里可直接核对结构。

## 前端

- 无。
