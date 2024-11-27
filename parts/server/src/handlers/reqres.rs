use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

#[axum::debug_handler]
pub async fn handler(State(state): State<crate::State>, body: Bytes) -> impl IntoResponse {
    println!("hello!");
    println!("{:?}", body);

    match match body[0] {
        0 => state.core.access_get(body),
        1 => state.core.group_assign(body),
        2 => state.core.group_create(body),
        3 => state.core.group_drop(body),
        4 => state.core.login_finish(body),
        5 => state.core.login_start(body),
        6 => state.core.registration_finish(body),
        7 => state.core.registration_start(body),
        8 => state.core.secret_get(body),
        9 => state.core.secret_put(body),
        10 => state.core.storage_put(body),
        11 => state.core.storage_query(body),
        _ => Err("unknown action".into()),
    } {
        Ok(val) => (StatusCode::OK, val),
        Err(err) => {
            log::error!("{err}");
            (StatusCode::INTERNAL_SERVER_ERROR, Bytes::new())
        }
    }
}
