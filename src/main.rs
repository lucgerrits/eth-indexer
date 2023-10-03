/// Eth-indexer is a tool for indexing Ethereum blocks and transactions.
/// It will index the blocks and transactions into a Postgres database for later
/// querying.
///
/// main.rs
use dotenv::dotenv;
use std::env;
mod blockscout;
mod db;
mod indexer;
mod indexer_types;
mod rpc;
use crate::indexer::Indexer;
pub use indexer_types::*;
use log::info;

/// This function is the entry point for the program.
/// It will start the indexer and begin indexing blocks and transactions.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    check_env();
    load_env();

    let indexer = Indexer::new();
    indexer.run().await?;

    info!("Done!");
    Ok(())
}

fn check_env() {
    dotenv().ok();
    info!("Configuration:");
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
    info!("");
}

fn load_env() {
    if env::var("LOG_LEVEL").is_err() {
        env::set_var("LOG_LEVEL", "info")
    }
    env_logger::Builder::from_env("LOG_LEVEL").init();
}
