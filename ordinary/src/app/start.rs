use std::sync::Arc;
use std::thread;

use log::info;

use saferlmdb::EnvBuilder;

use crate::optimizer::Optimizer;
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
    let graph_store = Arc::new(storage::StorageSystem::new(env)?);

    info!("{:?}", graph_store.stat());

    // operations
    let mut graph_put8 = GraphPut8::new(graph_store.clone());

    let optimizer = Optimizer::new(graph_put8.instruction_channel.clone());

    let handle = thread::spawn(move || {
        if let Err(err) = graph_put8.start() {
            println!("{:?}", err);
        }
    });

    thread::spawn(move || {
        if let Err(err) = optimizer.start() {
            log::error!("{}", err);
        }
    });

    handle.join().unwrap();

    Ok(())
}
