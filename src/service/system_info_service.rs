use anyhow::Result;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

use crate::dto::{SystemInfoQuery, SystemInfoResponse};
use crate::entities::guardian_systeminfo;
use crate::response::Response;
use crate::router::AppState;

pub async fn list_system_info_service(
    state: AppState,
    query: SystemInfoQuery,
) -> Result<Response<Vec<SystemInfoResponse>>> {
    let mut select =
        guardian_systeminfo::Entity::find().order_by_desc(guardian_systeminfo::Column::CreatedAt);

    if let Some(limit) = query.limit {
        select = select.limit(limit as u64);
    } else {
        select = select.limit(6);
    }

    let system_info_list = select
        .all(&state.conn)
        .await?
        .into_iter()
        .map(SystemInfoResponse::from)
        .collect();

    Ok(Response::ok_data(system_info_list))
}
