/// Module to handle RPC requests
use ethers::prelude::*;
use std::env;
use log::info;
/// Function to connect to the RPC endpoint in WebSocket
/// Returns a client
pub async fn connect_rpc() -> Provider<Ws> {
    let ws_endpoint = env::var("WS_RPC_ENDPOINT").unwrap();
    let client: Provider<Ws> = match Provider::<Ws>::connect(ws_endpoint.as_str()).await {
        Ok(client) => client,
        Err(e) => panic!("Error connecting to RPC endpoint: {}", e),
    };
    info!("Connected to RPC endpoint!");
    client
}
