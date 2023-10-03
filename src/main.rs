/// Eth-indexer is a tool for indexing Ethereum blocks and transactions.
/// It will index the blocks and transactions into a Postgres database for later
/// querying.
///
/// main.rs
use dotenv::dotenv;
use ethers::prelude::*;
use std::{env, sync::Arc};
mod indexer;
mod db;
mod rpc;
mod blockscout;
mod indexer_types;
pub use indexer_types::*;
use log::{error as log_error, info};

/// This function is the entry point for the program.
/// It will start the indexer and begin indexing blocks and transactions.
///
/// # Configuration
/// Use a .env file to configure the indexer. The following environment
/// variables are used:
/// HTTP_RPC_ENDPOINT - The HTTP RPC endpoint to connect to
/// POSTGRES_HOST="localhost"
/// DPOSTGRES_PORT="5432"
/// POSTGRES_USER="postgres"
/// POSTGRES_PASSWORD="postgres"
/// POSTGRES_DB="eth-indexer"
///
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Configuration:");
    dotenv().ok();
    // Check all the environment variables are set
    let env_vars = vec![
        "VERSION",
        "HTTP_RPC_ENDPOINT",
        "WS_RPC_ENDPOINT",
        "POSTGRES_HOST",
        "POSTGRES_PORT",
        "POSTGRES_USER",
        "POSTGRES_PASSWORD",
        "POSTGRES_DB",
        "POSTGRES_CREATE_TABLE_ORDER",
        "BATCH_SIZE",
        "START_BLOCK",
        "END_BLOCK",
        "LOG_LEVEL",
    ];
  
    for env_var in env_vars {
        match env::var(env_var) {
            Ok(_) => println!("{0: <30}= {1}", env_var, env::var(env_var).unwrap()),
            Err(_) => panic!("{} is not set", env_var),
        }
    }
    if env::var("LOG_LEVEL").is_err() {
        env::set_var("LOG_LEVEL", "info")
    }
    env_logger::Builder::from_env("LOG_LEVEL").init();
    info!("");

    // Connect to the database
    let db_pool = db::connect_db().await;
    // Connect to the RPC endpoint
    let ws_client = Arc::new(rpc::connect_rpc().await);

    // Init database
    if let Err(e) = db::init_db(db_pool.clone()).await {
        log_error!("Error initializing the database: {}", e);
    }

    // Get the latest block number
    let last_block = indexer::get_latest_block(ws_client.clone()).await?;
    info!("Latest block number: {}", last_block);

    // if START_BLOCK is set, use that as the start block
    let start_block = U64::from(
        env::var("START_BLOCK")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u64>()
            .unwrap_or(0),
    );
    // if END_BLOCK is set and different then -1, use that as the end block
    let end_block = U64::from(
        env::var("END_BLOCK")
            .unwrap_or_else(|_| "-1".to_string())
            .parse::<u64>()
            .unwrap_or(last_block.as_u64()),
    );

    info!("Starting indexing from block {} to {}", start_block, end_block);

    match indexer::index_blocks(U64::from(start_block), U64::from(end_block), ws_client.clone(), db_pool.clone()).await {
        Ok(_) => info!("Indexing complete!", ),
        Err(e) => log_error!("Error indexing blocks: {}", e),
    }

    info!("Done!");
    Ok(())
}
