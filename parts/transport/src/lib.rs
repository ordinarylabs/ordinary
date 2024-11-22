use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;

struct Server {}

impl Server {
    pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/", post(async_handler))
            .with_state(storage);
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn async_handler(
    State(storage): State<Arc<StorageSystem>>,
    body: Bytes,
) -> impl IntoResponse {
    let storage = storage.clone();

    let (tx, rx) = oneshot::channel();

    rayon::spawn(move || {
        let res = storage.query(body).unwrap();
        tx.send(res).unwrap();
    });

    let res = rx.await.unwrap();

    (StatusCode::OK, res)
}
