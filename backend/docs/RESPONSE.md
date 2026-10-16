# 统一响应结构

> 本文档详解所有带响应体接口的统一返回结构，接口清单见 [README](../README.md)。

所有带响应体的接口共用同一结构：

```json
{
  "code": 200,
  "message": "Success",
  "data": {},
  "traceId": "01a139adae5d7692970c3d74806c2ccb",
  "timestamp": 1713179523000
}
```

约定：

- `code` 由 HTTP 状态码派生，二者永远一致。`ApiResponse` 内部只保存 `StatusCode` 这一份真值，序列化时才写出 `code`。
- `data` 为空时该字段不会出现在 JSON 中。
- `traceId` 由 `src/middleware/trace.rs` 在响应出栈时注入：优先沿用请求头 `x-trace-id`，缺失时生成 32 位十六进制，
  同时写入响应头 `x-trace-id`；CSV / ZIP 等非 JSON 响应只加响应头，不改响应体。
- `timestamp` 为 Unix epoch 毫秒（UTC 基准，与时区无关）。
- 需要非 200 语义（例如 `201 Created`）时使用 `ApiResponse::with_status(StatusCode::CREATED, ...)`。
- `204 No Content` 这类不带响应体的状态码不属于 `ApiResponse` 的职责，由 handler 直接返回 `StatusCode`。
- 请求保护的 `408` / `413` / `429` / `503` 与 panic `500` 使用同一响应结构，由请求观测层保证：
  axum 生成的纯文本 400/404/405/413/415/422 会被归一化成稳定 JSON，不回显解析细节；
  已经是 JSON 的业务响应不改写，405 保留 `Allow` 响应头。
- 响应的 `x-request-id` 可关联服务日志；客户端速率拒绝带 `Retry-After`（秒），并发拒绝不猜测等待时间。
- 响应体里的 `traceId` 由改写 JSON 的那一层（压缩层之内）注入；由外层生成的响应体——
  浏览器的 gzip 响应、限流 `429`、背压 `503` 与 panic `500`——可能只带 `x-trace-id` 响应头。
  关联请求时以响应头为准。
- 以上约定只针对应用内错误；网关、HTTP 解析层、TLS 失败与响应流中断不保证有统一 JSON。
