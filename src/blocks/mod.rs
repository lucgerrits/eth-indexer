// Module that handle block indexing
// blocks/mod.rs
use crate::db;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use log::{info, warn};
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_postgres::NoTls;
// use bb8_postgres::tokio_postgres::Error as PostgresError;

/// Get the latest block number
pub async fn get_latest_block(ws_client: Arc<Provider<Ws>>) -> Result<U64, Box<dyn Error>> {
    match ws_client.get_block(BlockNumber::Latest).await {
        Ok(Some(Block {
            number: Some(block),
            ..
        })) => Ok(block),
        _ => Err("Error getting latest block".into()), // Convert the string into a Box<dyn Error>
    }
}

/// Index the blocks
/// We will index the blocks in parallel in batches of `BATCH_SIZE` blocks.
/// The batch size can be configured with the environment variable `BATCH_SIZE`.
///
/// A block is indexed by calling the `index_block` function.
/// A block contains a list of transactions. Each transaction is indexed by
/// calling the `index_transaction` function.
///
pub async fn index_blocks(
    start_block: U64,
    end_block: U64,
    ws_client: Arc<Provider<Ws>>,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), String> {
    let batch_size = U64::from(
        env::var("BATCH_SIZE")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u64>()
            .unwrap_or(10),
    );

    let mut batch_start = start_block;
    let mut batch_end = batch_start + batch_size;

    let total_blocks = end_block.as_u64() - start_block.as_u64();
    let mut blocks_processed = 0;
    let mut blocks_processed_total = 0;
    let mut start_time = Instant::now();

    while batch_end <= end_block {
        // println!("Indexing blocks {} to {}", batch_start, batch_end);
        
        let mut tasks = vec![];

        for block_number in batch_start.as_u64()..batch_end.as_u64() {
            let thd_ws_client = Arc::clone(&ws_client);
            let thd_db_pool = db_pool.clone(); // Clone the connection pool for each thread
            let thd_block_number = block_number.clone();

            tasks.push(tokio::spawn(async move {
                index_block(U64::from(thd_block_number), thd_ws_client, &thd_db_pool).await
            }));
        }

        for task in tasks {
            if let Err(e) = task.await {
                eprintln!("Error indexing blocks: {:?}", e);
            }
        }

        batch_start += batch_size;
        batch_end += batch_size;

        // Calculate blocks per second and log it every 10 seconds
        blocks_processed += batch_size.as_u64();
        blocks_processed_total += batch_size.as_u64();
        let elapsed_time = start_time.elapsed();
        if elapsed_time >= Duration::new(3, 0) {
            info!(
                "Indexing blocks {:.2}%",
                (blocks_processed_total as f64 / total_blocks as f64 * 100.0)
            );
    
            let blocks_per_second =
                blocks_processed as f64 / elapsed_time.as_secs_f64();
            info!("Blocks per second: {:.2}", blocks_per_second);
            start_time = Instant::now();
            blocks_processed = 0;
        }
    }

    // Index the remaining blocks
    // if batch_start < end_block {
    //     println!("Indexing blocks {} to {}", batch_start, end_block);

    //     let mut tasks = vec![];

    //     for block_number in batch_start.as_u64()..batch_end.as_u64() {
    //         let ws_client = Arc::clone(&ws_client);
    //         let db_client = Arc::clone(&db_client);

    //         tasks.push(tokio::spawn(async move {
    //             index_block(block_number, ws_client, db_client).await
    //         }));
    //     }

    //     for task in tasks {
    //         if let Err(e) = task.await {
    //             eprintln!("Error indexing blocks: {:?}", e);
    //         }
    //     }
    // }

    Ok(())
}

/// Index a block
/// A block contains a list of transactions. Each transaction is indexed by
/// calling the `index_transaction` function.
async fn index_block(
    block_number: U64,
    ws_client: Arc<Provider<Ws>>,
    db_pool: &Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), String> {
    match ws_client.get_block(block_number).await {
        Ok(Some(block)) => {
            // Index block
            if let Err(e) = db::insert_block(block.clone(), db_pool.to_owned()).await {
                let error_message = format!("Error inserting block into database: {:?}", e);
                eprintln!("{}", error_message);
                return Err(error_message); // Return the error message
            }

            // Index transactions only after inserting the block
            for transaction_hash in block.transactions {
                let ws_client = Arc::clone(&ws_client);
                let thd_db_pool = db_pool.clone(); // Clone the connection pool for each thread

                if let Err(e) = index_transaction(transaction_hash, ws_client, &thd_db_pool).await {
                    let error_message = format!("Error indexing transactions: {:?}", e);
                    eprintln!("{}", error_message);
                }
            }
        }
        _ => eprintln!("Error indexing block {}", block_number),
    }

    Ok(())
}

/// Index a transaction
async fn index_transaction(
    transaction_hash: TxHash,
    ws_client: Arc<Provider<Ws>>,
    db_pool: &Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), String> {
    match ws_client.get_transaction(transaction_hash).await {
        Ok(Some(transaction)) => {
            // Index transaction
            if let Err(e) = db::insert_transaction(transaction, db_pool.clone()).await {
                let error_message = format!("Error inserting transaction into database: {:?}", e);
                eprintln!("{}", error_message);
                return Err(error_message); // Return the error message
            }
        }
        _ => eprintln!("Error indexing transaction {}", transaction_hash),
    }

    Ok(())
}
