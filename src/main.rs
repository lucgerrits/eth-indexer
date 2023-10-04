/// Eth-indexer is a tool for indexing Ethereum blocks and transactions.
/// It will index the blocks and transactions into a Postgres database for later
/// querying.
///
/// main.rs
use std::{env, time::Duration};
mod blockscout;
mod db;
mod indexer;
mod indexer_types;
mod rpc;
use crate::indexer::Indexer;
pub use indexer_types::*;
use log::{info, warn};
use tokio::signal;

/// This function is the entry point for the program.
/// It will start the indexer and begin indexing blocks and transactions.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    check_env();
    load_env();
    let args: Vec<String> = env::args().collect();

    match args.len() {
        // no arguments passed
        1 => {
            help();
        }
        // one argument passed
        2 => match args[1].as_str() {
            "index_all" => {
                warn!("Starting indexer");
                let indexer = Indexer::new().await;
                indexer.run().await?;
            }
            "index_live" => {
                warn!("Starting live indexer");
                let indexer = Indexer::new().await;

                // Register a signal handler for CTRL+C (SIGINT)
                let ctrl_c = signal::ctrl_c();

                // Create a future that completes after a timeout
                let timeout = tokio::time::sleep(Duration::from_secs(10));

                tokio::select! {
                    _ = ctrl_c => {
                        // Handle the exit signal here
                        println!("\nReceived exit signal. Exiting...");
                    }
                    _ = indexer.run_live() => {}
                    _ = timeout => {
                        // Handle timeout if needed
                    }
                }
            }
            "help" | "--help" | "-h" | "-v" | "--version"  => {
                help();
            }
            _ => {
                println!("'{}' is not a valid argument", args[1]);
                help();
            }
        },
        // three arguments passed
        3 => match args[1].as_str() {
            "index_last" => {
                warn!("Starting indexer");
                let indexer = Indexer::new().await;
                let number_of_blocks: u64 = args[2].parse().unwrap();
                indexer.run_last_blocks(number_of_blocks).await?;
            }
            _ => {
                println!("'{}' is not a valid argument", args[1]);
                help();
            }
        },
        _ => {
            println!("Too many arguments passed");
            help();
        }
    }
    Ok(())
}

fn help() {
    println!("\nUsage: eth-indexer [index_all|index_live|help]\n");
    let version = env!("CARGO_PKG_VERSION");
    println!("eth-indexer v{}", version);
}

fn check_env() {
    // Determine which environment file to load
    let env_file = match env::var("ETH_INDEXER") {
        Ok(value) if value == "production" => {
            println!("Using .env.production for configuration.");
            ".env.production"
        }
        _ => {
            println!("Using .env for configuration.");
            ".env"
        }
    };

    dotenv::from_filename(env_file).ok();

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
        "POSTGRES_DATABASE",
        "POSTGRES_CREATE_TABLE_ORDER",
        "BATCH_SIZE",
        "LOG_LEVEL",
    ];

    for env_var in env_vars {
        match env::var(env_var) {
            Ok(_) => println!("{0: <30}= {1}", env_var, env::var(env_var).unwrap()),
            Err(_) => panic!("{} is not set", env_var),
        }
    }
    println!("");
}

fn load_env() {
    if env::var("LOG_LEVEL").is_err() {
        env::set_var("LOG_LEVEL", "info")
    }
    env_logger::Builder::from_env("LOG_LEVEL").init();
}
