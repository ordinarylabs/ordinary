use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use log::info;
use tokio::runtime::Builder;
use tokio::sync::oneshot;

use crate::storage::StorageSystem;

pub enum Instruction7 {
    StartPort(u16),
    AddPort(u16),
    WindDownPort(u16),
    KillPort(u16),
}

pub struct GraphQuery7 {
    storage: Arc<StorageSystem>,

    instructions_listener: flume::Receiver<Instruction7>,
    pub instruction_channel: flume::Sender<Instruction7>,

    port_handles: BTreeMap<u16, JoinHandle<()>>,
}

impl GraphQuery7 {
    pub fn new(storage: Arc<StorageSystem>) -> Self {
        let (instruction_tx, instruction_rx) = flume::unbounded();

        Self {
            storage,

            instruction_channel: instruction_tx,
            instructions_listener: instruction_rx,

            port_handles: BTreeMap::new(),
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok(instruction) = self.instructions_listener.recv() {
            match instruction {
                Instruction7::StartPort(port) => {
                    let storage = self.storage.clone();

                    let handle = thread::spawn(move || {
                        info!("started on port {}", port);

                        let runtime = Builder::new_current_thread().enable_io().build().unwrap();
                        runtime.block_on(serve_http(port, storage)).unwrap();
                    });

                    self.port_handles.insert(port, handle);
                }
                Instruction7::AddPort(port) => (),
                Instruction7::WindDownPort(port) => (),
                Instruction7::KillPort(port) => (),
            };
        }
        Ok(())
    }
}

async fn serve_http(
    port: u16,
    storage: Arc<StorageSystem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", post(async_handler))
        .with_state(storage);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    axum::serve(listener, app).await?;

    Ok(())
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
