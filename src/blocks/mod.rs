// Module that handle block indexing
use ethers::{
    core::{
        types::{Block, BlockNumber, U64},
    },
    providers::{Middleware, Provider, Ws},
};
use std::sync::Arc;
use std::error::Error;

pub async fn get_latest_block(ws_client: Arc<Provider<Ws>>) -> Result<U64, Box<dyn Error>> {
    match ws_client.get_block(BlockNumber::Latest).await {
        Ok(Some(Block { number: Some(block), .. })) => Ok(block),
        _ => Err("Error getting latest block".into()), // Convert the string into a Box<dyn Error>
    }
}