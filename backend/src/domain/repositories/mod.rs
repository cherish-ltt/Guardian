//! 领域层不放仓储接口实现。
//!
//! 仓储方法需要接收「可复用的连接」（`DatabaseConnection` 或 `DatabaseTransaction`），
//! 因此接口签名会直接引用 `sea_orm::ConnectionTrait`。为了守住
//! 「`domain` 不依赖 sea-orm」这条边界，仓储接口与实现一起放在
//! `src/infrastructure/repositories`，领域层只保留 `domain/services` 中的用例接口。
//!
//! 参考实现见 `infrastructure::repositories::transaction::TransferRepository`。
