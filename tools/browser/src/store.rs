// std::fs::create_dir_all("./.stewball")?;

//         info!("starting...");

// let env = unsafe {
//             let mut env_builder = EnvBuilder::new().unwrap();
//             env_builder.set_maxreaders(126).unwrap();
//             env_builder.set_mapsize(10485760_0).unwrap();
//             env_builder
//                 .open("./store", saferlmdb::open::Flags::empty(), 0o600)
//                 .unwrap()
//         };

// let group_db = Database::open(
//             env,
//             Some("group"),
//             &DatabaseOptions::new(
//                 lmdb::db::Flags::DUPSORT | lmdb::db::Flags::DUPFIXED | lmdb::db::Flags::CREATE,
//             ),
//         )?;

// let graph_db = Database::open(
//             env.clone(),
//             Some("graph"),
//             &DatabaseOptions::new(lmdb::db::Flags::CREATE),
//         )?;
