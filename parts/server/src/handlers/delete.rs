use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

#[axum::debug_handler]
pub async fn handler(
    State(state): State<crate::State>,
    Path(crate::ReqResPath { entity, uuid }): Path<crate::ReqResPath>,
    body: Bytes,
) -> impl IntoResponse {
    println!("entity: {entity}, uuid: {uuid}");
    (StatusCode::OK, body)
}
