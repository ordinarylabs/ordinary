use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use reqwest;
use saferlmdb::EnvBuilder;
use stewball::StorageSystem;
use tokio::net::TcpListener;

use hostess;

async fn server() -> Result<(SocketAddr, SocketAddr), Box<dyn std::error::Error>> {
    let reqres_listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let reqres_addr = reqres_listener.local_addr().unwrap();

    let stream_listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let stream_addr = stream_listener.local_addr().unwrap();

    std::fs::create_dir_all("./store")?;

    let env = Arc::new(unsafe {
        let mut env_builder = EnvBuilder::new().unwrap();
        env_builder.set_maxreaders(126).unwrap();
        env_builder.set_mapsize(10485760_0).unwrap();
        env_builder
            .open("./store", saferlmdb::open::Flags::empty(), 0o600)
            .unwrap()
    });

    let router = BTreeMap::new();
    let storage = StorageSystem::new(env)?;

    tokio::spawn(async move {
        hostess::start(reqres_listener, stream_listener, "./", router, storage)
            .await
            .unwrap();
    });

    Ok((reqres_addr, stream_addr))
}

#[tokio::test]
async fn delete() -> Result<(), Box<dyn std::error::Error>> {
    let (reqres_addr, _) = server().await?;
    let client = reqwest::Client::new();

    let res = client
        .delete(format!("http://{reqres_addr}/hi/friend/its/me"))
        .body("Hello world!")
        .send()
        .await?
        .bytes()
        .await?;

    assert_eq!(&res[..], b"Hello world!");
    Ok(())
}

#[tokio::test]
async fn get() -> Result<(), Box<dyn std::error::Error>> {
    let (reqres_addr, _) = server().await?;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("http://{reqres_addr}/hi/friend/its/me"))
        .body("Hello world!")
        .send()
        .await?
        .bytes()
        .await?;

    assert_eq!(&res[..], b"Hello world!");
    Ok(())
}

#[tokio::test]
async fn put() -> Result<(), Box<dyn std::error::Error>> {
    let (reqres_addr, _) = server().await?;
    let client = reqwest::Client::new();

    let res = client
        .put(format!("http://{reqres_addr}/hi/friend/its/me"))
        .body("Hello world!")
        .send()
        .await?
        .bytes()
        .await?;

    assert_eq!(&res[..], b"Hello world!");
    Ok(())
}
