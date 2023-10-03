// Module: db::blocks

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use rust_decimal::prelude::*;
use serde_json;
use std::error::Error;
use tokio_postgres::{types::ToSql, NoTls};
use log::{error as log_error, debug};

/// Function to insert a block into the database
/// Database schema:
/// CREATE TABLE blocks (
/// "number" BIGINT NOT NULL PRIMARY KEY,
/// "hash" VARCHAR(66) NOT NULL,
/// "parentHash" VARCHAR(66) NOT NULL,
/// "nonce" VARCHAR(18) NOT NULL,
/// "sha3Uncles" VARCHAR(66) NOT NULL,
/// "logsBloom" TEXT NOT NULL,
/// "transactionsRoot" VARCHAR(66) NOT NULL,
/// "stateRoot" VARCHAR(66) NOT NULL,
/// "miner" VARCHAR(42) NOT NULL,
/// "difficulty" BIGINT NOT NULL,
/// "totalDifficulty" NUMERIC(50),
/// "size" INT NOT NULL,
/// "extraData" VARCHAR(66) NOT NULL,
/// "gasLimit" NUMERIC(100),
/// "gasUsed" NUMERIC(100),
/// "timestamp" INT NOT NULL,
/// "transactionsCount" INT,
/// "transactions_ids" JSON,
/// "uncles" JSON,
/// "lastUpdated" timestamp default current_timestamp
/// );
///
pub async fn insert_block(
    block: Block<H256>,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    debug!(
        "Inserting block {} into database",
        block.number.unwrap().to_string()
    );

    // Extract relevant data from the block
    let number = block.number.unwrap().as_u64() as i64;
    let hash = format!("0x{:x}", block.hash.unwrap());
    let parent_hash = format!("0x{:x}", block.parent_hash);
    let nonce = format!("0x{:x}", block.nonce.unwrap());
    let sha3_uncles = serde_json::to_value(&block.uncles).unwrap().to_string();
    let logs_bloom = format!("0x{:x}", block.logs_bloom.unwrap());
    let transactions_root = format!("0x{:x}", block.transactions_root);
    let state_root = format!("0x{:x}", block.state_root);
    let miner = format!("0x{:x}", block.author.unwrap());
    let difficulty = block.difficulty.as_u64() as i64;
    // let total_difficulty = block.total_difficulty.map(|d| Decimal::from(d.as_u64() as i64)).unwrap_or_default();
    let total_difficulty = block
        .total_difficulty
        .map(|u256| Decimal::from(u256.as_u128()))
        .unwrap_or(Decimal::new(0, 0));
    let size = block.size.unwrap().as_u32() as i32;
    let extra_data = format!("{:x}", block.extra_data);
    let gas_limit = Decimal::from(block.gas_limit.as_u128() as i64);
    let gas_used = Decimal::from(block.gas_used.as_u128() as i64);
    let timestamp = block.timestamp.as_u64() as i32;
    let transactions_count = block.transactions.len() as i32;
    let transactions_ids = serde_json::to_value(&block.transactions).unwrap();
    let uncles = serde_json::to_value(&block.uncles).unwrap();

    // Build the SQL query
    let query = r#"
        INSERT INTO blocks ("number", "hash", "parentHash", "nonce", "sha3Uncles", "logsBloom", "transactionsRoot",
                            "stateRoot", "miner", "difficulty", "totalDifficulty", "size", "extraData", "gasLimit",
                            "gasUsed", "timestamp", "transactionsCount", "transactions_ids", "uncles", "insertedAt")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, NOW())
        ON CONFLICT ("number") DO NOTHING;
    "#;
    // Prepare the statement
    let db_client = db_pool.get().await.map_err(|e| {
        log_error!("Error acquiring database connection: {:?}", e);
        Box::new(e) as Box<dyn Error>
    })?;
    let statement = db_client
        .prepare(query)
        .await
        .expect("Failed to prepare statement");

    // Prepare the parameter values
    let params: [&(dyn ToSql + Sync); 19] = [
        &number,
        &hash,
        &parent_hash,
        &nonce,
        &sha3_uncles,
        &logs_bloom,
        &transactions_root,
        &state_root,
        &miner,
        &difficulty,
        &total_difficulty,
        &size,
        &extra_data,
        &gas_limit,
        &gas_used,
        &timestamp,
        &transactions_count,
        &transactions_ids,
        &uncles,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            debug!("Block {} inserted successfully", number);
            Ok(())
        }
        Err(err) => {
            log_error!("Error inserting block {}: {}", number, err);
            Err(Box::new(err))
        }
    }
}
