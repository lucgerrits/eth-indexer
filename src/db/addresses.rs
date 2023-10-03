// Module: db::addresses

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use rust_decimal::prelude::*;
use std::error::Error;
use tokio_postgres::{types::ToSql, NoTls};
use log::{error as log_error, debug};

/// Function to insert an address into the database
/// Update the address if it already exists with this rules:
/// - If the block number is the same, do nothing
/// - If the block number is higher, update all fields
/// - If the block number is lower, do nothing
/// - If the block number is missing, do nothing
/// Database schema:
/// CREATE TABLE addresses (
/// "address" VARCHAR(42) NOT NULL PRIMARY KEY,
/// "balance" NUMERIC(100),
/// "nonce" INT,
/// "transactionCount" INT,
/// "blockNumber" BIGINT NOT NULL,
/// "contractCode" TEXT,
/// "gasUsed" INT,
/// "storage" VARCHAR(66),
/// "tokens" JSON,
/// "lastUpdated" timestamp default current_timestamp,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE
/// );
pub async fn insert_address(
    address: Address,
    balance: U256,
    nonce: U256,
    transaction_count: U256,
    storage: H256,
    code: Bytes,
    block_number: U64,
    _gas_used: U256, //TODO: handle the gas usage of an address
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the address
    let address = format!("0x{:x}", address);
    let balance = Decimal::from_parts(
        balance.low_u32() as u32, // lo
        0,                        // mid
        0,                        // hi
        false,                    // negative
        0,                        // scale
    );
    let nonce = nonce.as_u64() as i32;
    let transaction_count = transaction_count.as_u64() as i32;
    let storage = format!("0x{:x}", storage);
    let block_number = block_number.as_u64() as i64;
    let code = format!("{:x}", code);

    // Build the SQL query
    let query = r#"
        INSERT INTO addresses ("address", "balance", "nonce", "transactionCount", "blockNumber",
                               "contractCode", "storage", "insertedAt")
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        ON CONFLICT ("address") DO UPDATE
        SET "balance" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."balance"
            ELSE addresses."balance"
            END,
            "nonce" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."nonce"
            ELSE addresses."nonce"
            END,
            "transactionCount" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."transactionCount"
            ELSE addresses."transactionCount"
            END,
            "blockNumber" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."blockNumber"
            ELSE addresses."blockNumber"
            END,
            "contractCode" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."contractCode"
            ELSE addresses."contractCode"
            END,
            "storage" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."storage"
            ELSE addresses."storage"
            END
        WHERE excluded."blockNumber" IS NOT NULL
        AND excluded."blockNumber" > addresses."blockNumber";
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
    let params: [&(dyn ToSql + Sync); 7] = [
        &address,
        &balance,
        &nonce,
        &transaction_count,
        &block_number,
        &code,
        &storage,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            debug!("Address {} inserted/updated successfully", address);
            Ok(())
        }
        Err(err) => {
            log_error!("Error inserting/updating address {}: {}", address, err);
            Err(Box::new(err))
        }
    }
}