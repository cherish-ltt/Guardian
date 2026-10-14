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
