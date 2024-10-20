use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::put;
use axum::Router;
use log::info;
use tokio::runtime::Builder;

use crate::storage::StorageSystem;

pub enum Instruction8 {
    StartPort(u16),
    AddPort(u16),
    WindDownPort(u16),
    KillPort(u16),
}

pub struct GraphPut8 {
    storage: Arc<StorageSystem>,

    instructions_listener: flume::Receiver<Instruction8>,
    pub instruction_channel: flume::Sender<Instruction8>,

    port_handles: BTreeMap<u16, JoinHandle<()>>,
}

impl GraphPut8 {
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
                Instruction8::StartPort(port) => {
                    let storage = self.storage.clone();

                    let handle = thread::spawn(move || {
                        info!("started on port {}", port);

                        let runtime = Builder::new_current_thread().enable_io().build().unwrap();
                        runtime.block_on(serve_http(port, storage)).unwrap();
                    });

                    self.port_handles.insert(port, handle);
                }
                Instruction8::AddPort(port) => (),
                Instruction8::WindDownPort(port) => (),
                Instruction8::KillPort(port) => (),
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
        .route("/", put(async_handler))
        .with_state(storage);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn async_handler(
    State(storage): State<Arc<StorageSystem>>,
    body: Bytes,
) -> impl IntoResponse {
    match storage.put(body) {
        Ok(res) => (StatusCode::OK, res),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string().into()),
    }
}
