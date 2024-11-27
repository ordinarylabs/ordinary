mod handlers;
use handlers::{reqres, stream};

use axum::routing::{any, delete, get, put};
use axum::Router;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use std::collections::BTreeMap;
use std::net::SocketAddr;

use serde::Deserialize;
use uuid::Uuid;

use stewball::{self, Core};

#[derive(Clone)]
struct State {
    router: BTreeMap<Vec<u8>, Vec<u8>>,
    core: stewball::Core,
}

#[derive(Deserialize)]
struct ReqResPath {
    parent: Uuid,
    entity: String,
    uuid: Uuid,
}

fn reqres(state: State, assets_dir: &str) -> Router {
    Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/", any(reqres::handler))
        .with_state(state)
}

fn stream(state: State) -> Router {
    Router::new()
        .route("/", any(stream::handler))
        .with_state(state.clone())
}

pub async fn start(
    reqres_listener: TcpListener,
    stream_listener: TcpListener,
    assets_dir: &str,
    router: BTreeMap<Vec<u8>, Vec<u8>>,
    core: Core,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = State { router, core };

    let (reqres, stream) = tokio::join!(
        axum::serve(reqres_listener, reqres(state.clone(), assets_dir)),
        axum::serve(
            stream_listener,
            stream(state).into_make_service_with_connect_info::<SocketAddr>()
        )
    );

    reqres?;
    stream?;

    Ok(())
}
