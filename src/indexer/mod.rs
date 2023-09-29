// Module that handle block indexing
// blocks/mod.rs
use crate::{db, blockscout};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use log::info;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_postgres::NoTls;

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
    let mut start_time: Instant = Instant::now();

    while batch_end <= end_block {
        // println!("Indexing blocks {} to {}", batch_start, batch_end);

        let mut tasks = vec![];

        for block_number in batch_start.as_u64()..batch_end.as_u64() {
            //skip if block_number is > end_block
            if block_number > end_block.as_u64() {
                continue;
            }
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

        // Calculate stats and log it every 10 seconds
        blocks_processed += batch_size.as_u64();
        blocks_processed_total += batch_size.as_u64();
        let elapsed_time = start_time.elapsed();
        if elapsed_time >= Duration::new(3, 0) {
            let progress = blocks_processed_total as f64 / total_blocks as f64 * 100.0;

            // Calculate estimated remaining time
            let elapsed_seconds = elapsed_time.as_secs_f64();
            let remaining_blocks = total_blocks - blocks_processed_total;
            let estimated_remaining_time_secs = if blocks_processed > 0 {
                (remaining_blocks as f64 / blocks_processed as f64) * elapsed_seconds
            } else {
                0.0
            };
            let estimated_remaining_time = Duration::from_secs_f64(estimated_remaining_time_secs);

            info!("Indexing blocks {:.1}%", progress);
            info!(
                "Blocks per second: {:.1}",
                blocks_processed as f64 / elapsed_seconds
            );
            info!(
                "Estimated remaining time (sec): {:.1}",
                estimated_remaining_time.as_secs_f32()
            );

            start_time = Instant::now();
            blocks_processed = 0;
        }
    }

    // Index the remaining blocks
    if batch_start < end_block {
        let mut tasks = vec![];

        for block_number in batch_start.as_u64()..batch_end.as_u64() {
            //skip if block_number is > end_block
            if block_number > end_block.as_u64() {
                continue;
            }
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
    }

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
            if let Err(e) = db::insert_transaction(transaction.clone(), db_pool.clone()).await {
                let error_message = format!("Error inserting transaction into database: {:?}", e);
                eprintln!("{}", error_message);
                return Err(error_message); // Return the error message
            }
            // Index the from address
            if let Err(e) = index_address(
                transaction.from,
                transaction.block_number.unwrap(),
                ws_client.clone(),
                db_pool.clone(),
            )
            .await
            {
                let error_message = format!("Error indexing address: {:?}", e);
                eprintln!("{}", error_message);
                return Err(error_message); // Return the error message
            }
            // Index the to address, if address is not zero
            if transaction.to.unwrap_or(Address::zero()) != Address::zero() {
                if let Err(e) = index_address(
                    transaction.to.unwrap(),
                    transaction.block_number.unwrap(),
                    ws_client.clone(),
                    db_pool.clone(),
                )
                .await
                {
                    let error_message = format!("Error indexing address: {:?}", e);
                    eprintln!("{}", error_message);
                    return Err(error_message); // Return the error message
                }
            }
            // Get the transaction receipt
            match ws_client.get_transaction_receipt(transaction_hash).await {
                Ok(Some(transaction_receipt)) => {
                    // Index transaction receipt
                    if let Err(e) =
                        db::insert_transaction_receipt(transaction_receipt.clone(), db_pool.clone())
                            .await
                    {
                        let error_message =
                            format!("Error inserting transaction receipt into database: {:?}", e);
                        eprintln!("{}", error_message);
                        return Err(error_message); // Return the error message
                    }
                    // Index the contract
                    if let Some(contract_address) = transaction_receipt.contract_address {
                        // Index the contract address
                        if let Err(e) = index_address(
                            contract_address,
                            transaction.block_number.unwrap(),
                            ws_client.clone(),
                            db_pool.clone(),
                        )
                        .await
                        {
                            let error_message = format!("Error indexing contract address: {:?}", e);
                            eprintln!("{}", error_message);
                            return Err(error_message); // Return the error message
                        }
                        // Index the smart contract (code and verified source code)
                        if let Err(e) = index_smart_contract(
                            transaction_receipt.clone(),
                            ws_client.clone(),
                            db_pool.clone(),
                        )
                        .await
                        {
                            let error_message = format!("Error indexing smart contract code: {:?}", e);
                            eprintln!("{}", error_message);
                            return Err(error_message); // Return the error message
                        }
                    }
                }
                _ => {
                    eprintln!("Error getting transaction receipt {}", transaction_hash);
                    return Ok(()); // Return the error message
                }
            };
        }
        _ => eprintln!("Error indexing transaction {}", transaction_hash),
    }

    Ok(())
}

/// Index an address
/// Here we have to :
/// - get the balance of the address
/// - get the code of the address (if it is a contract)
/// - get the storage of the address (if it is a contract)
/// - get transaction count
async fn index_address(
    address: Address,
    block_number: U64,
    ws_client: Arc<Provider<Ws>>,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), String> {
    let block_id = BlockId::from(BlockNumber::from(block_number.clone()));
    // Get the balance of the address
    let balance = match ws_client.get_balance(address, Some(block_id.clone())).await {
        Ok(balance) => balance,
        Err(e) => {
            eprintln!("Error getting balance for address {}: {}", address, e);
            return Err(e.to_string());
        }
    };
    // Get the code of the address (if it is a contract)
    let code = match ws_client.get_code(address, Some(block_id.clone())).await {
        Ok(code) => code,
        Err(_e) => {
            // eprintln!("Error getting code for address {}: {}", address, _e);
            // return Err(_e.to_string());
            Bytes::new() //it is possible that the address is not a contract
        }
    };

    // Get the storage of the address (if it is a contract)
    let storage = match ws_client
        .get_storage_at(address.to_string(), TxHash::zero(), Some(block_id.clone()))
        .await
    {
        Ok(storage) => storage,
        Err(_e) => {
            // eprintln!("Error getting storage for address {}: {}", address, _e);
            // return Err(_e.to_string());
            H256::zero() //it is possible that the address is not a contract
        }
    };

    // Get transaction count
    // Get the nounce of the address
    let transaction_count = match ws_client
        .get_transaction_count(address, Some(block_id.clone()))
        .await
    {
        Ok(count) => count,
        Err(e) => {
            eprintln!(
                "Error getting transaction count for address {}: {}",
                address, e
            );
            return Err(e.to_string());
        }
    };

    // Insert the address into the database
    if let Err(e) = db::insert_address(
        address,
        balance,
        transaction_count.clone(),
        transaction_count.clone(),
        storage,
        code,
        block_number.clone(),
        U256::from(0), //TODO: fix this
        db_pool.clone(),
    )
    .await
    {
        let error_message = format!("Error indexing transactions: {:?}", e);
        eprintln!("{}", error_message);
    }
    Ok(())
}

/// Index smart contract
/// We have to:
/// - get the code of the address
/// - Get the verified source code of the contract
async fn index_smart_contract(
    transaction_receipt: TransactionReceipt,
    ws_client: Arc<Provider<Ws>>,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), String> {
    // Get the code of the address (if it is a contract)
    let code = match ws_client
        .get_code(transaction_receipt.contract_address.unwrap(), None)
        .await
    {
        Ok(code) => code,
        Err(_e) => {
            // eprintln!("Error getting code for address {}: {}", address, _e);
            // return Err(_e.to_string());
            Bytes::new() //it is possible that the address is not a contract
        }
    };

    // Get the verified source code of the contract
    // TODO: get the verified source code using blockscout API
    let verified_source_code = blockscout::get_verified_source_code(format!("0x{:x}", transaction_receipt.contract_address.unwrap())).await;
    // let verified_source_code = serde_json::json!({});

    // Insert the address into the database
    if let Err(e) = db::insert_smart_contract(
        transaction_receipt,
        code,
        verified_source_code,
        db_pool.clone(),
    )
    .await
    {
        let error_message = format!("Error indexing transactions: {:?}", e);
        eprintln!("{}", error_message);
    }
    Ok(())
}
