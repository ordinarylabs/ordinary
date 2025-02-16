use std::sync::Arc;
use std::thread;

use log::info;

use saferlmdb::EnvBuilder;

use crate::orchestrator::Orchestrator;
use crate::paths::o7::GraphQuery7;
use crate::paths::o8::GraphPut8;
use crate::storage;

pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("./store")?;

    info!("starting");

    let env = Arc::new(unsafe {
        let mut env_builder = EnvBuilder::new().unwrap();
        env_builder.set_maxreaders(126).unwrap();
        env_builder.set_mapsize(10485760_0).unwrap();
        env_builder
            .open("./store", saferlmdb::open::Flags::empty(), 0o600)
            .unwrap()
    });

    // stores
    let storage_system = Arc::new(storage::StorageSystem::new(env)?);

    info!("{:?}", storage_system.stat());

    // operations
    let mut graph_query7 = GraphQuery7::new(storage_system.clone());
    let mut graph_put8 = GraphPut8::new(storage_system);

    let optimizer = Orchestrator::new(graph_put8.instruction_channel.clone());

    let query7_handle = thread::spawn(move || {
        if let Err(err) = graph_query7.start() {
            println!("{:?}", err);
        }
    });

    let put8_handle = thread::spawn(move || {
        if let Err(err) = graph_put8.start() {
            log::error!("{:?}", err);
        }
    });

    thread::spawn(move || {
        if let Err(err) = optimizer.start() {
            log::error!("{}", err);
        }
    });

    put8_handle.join().unwrap();

    Ok(())
}
