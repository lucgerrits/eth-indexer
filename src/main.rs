/// Eth-indexer is a tool for indexing Ethereum blocks and transactions.
/// It will index the blocks and transactions into a Postgres database for later
/// querying.
use dotenv::dotenv;
use std::{env, sync::Arc};
mod blocks;
mod db;
mod rpc;

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
    println!("Lets go!");
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
    ];

    for env_var in env_vars {
        match env::var(env_var) {
            Ok(_) => println!("{0: <30}= {1}", env_var, env::var(env_var).unwrap()),
            Err(_) => panic!("{} is not set", env_var),
        }
    }
    println!("");

    // Connect to the database
    let db_client = db::connect_db().await;
    // Connect to the RPC endpoint
    let ws_client = rpc::connect_rpc().await;

    // Init database
    if let Err(e) = db::init_db(Arc::new(db_client)).await {
        eprintln!("Error initializing the database: {:?}", e);
    }

    // Get the latest block number
    let last_block = blocks::get_latest_block(Arc::new(ws_client)).await?;
    println!("Latest block number: {}", last_block);

    println!("Done!");
    Ok(())
}
