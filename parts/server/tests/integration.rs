use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use hostess;
use saferlmdb::EnvBuilder;

async fn server() -> Result<(SocketAddr, SocketAddr), Box<dyn std::error::Error>> {
    let reqres_listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let reqres_addr = reqres_listener.local_addr().unwrap();

    let stream_listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let stream_addr = stream_listener.local_addr().unwrap();

    let router = BTreeMap::new();
    let core = stewball::Core::new()?;

    tokio::spawn(async move {
        hostess::start(reqres_listener, stream_listener, "./", router, core)
            .await
            .unwrap();
    });

    Ok((reqres_addr, stream_addr))
}

#[tokio::test]
async fn storage_put() -> Result<(), Box<dyn std::error::Error>> {
    let (reqres_addr, _) = server().await?;

    // register

    // login

    // generate access token from refresh token
    // let access_token = stewball::ops::access_get::new()?;

    // generate put request
    // let storage_put_req = stewball::ops::storage_put::new()?;

    // let client = reqwest::Client::new();

    // let res = client
    //     .put(format!("http://{reqres_addr}/"))
    //     .body(storage_put_req)
    //     .send()
    //     .await?
    //     .bytes()
    //     .await?;

    // assert_eq!(&res[..], b"Hello world!");
    Ok(())
}
