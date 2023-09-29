// Module: db::transactions

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use rust_decimal::prelude::*;
use serde_json;
use std::error::Error;
use tokio_postgres::{types::ToSql, NoTls};

/// Function to insert a transaction into the database
/// Database schema:
/// CREATE TABLE transactions (
/// r VARCHAR(66) NOT NULL,
/// s VARCHAR(66) NOT NULL,
/// v VARCHAR(4) NOT NULL,
/// "to" VARCHAR(42),
/// "gas" INT NOT NULL,
/// "from" VARCHAR(42) NOT NULL,
/// "hash" VARCHAR(66) NOT NULL PRIMARY KEY,
/// "type" SMALLINT NOT NULL,
/// "input" TEXT NOT NULL,
/// "nonce" INT NOT NULL,
/// "value" NUMERIC(100),
/// "chainId" VARCHAR(10),
/// "gasPrice" NUMERIC(100),
/// "blockHash" VARCHAR(66),
/// "accessList" JSON,
/// "blockNumber" BIGINT NOT NULL,
/// "maxFeePerGas" NUMERIC(100),
/// "transactionIndex" INT NOT NULL,
/// "maxPriorityFeePerGas" NUMERIC(100),
/// "lastUpdated" timestamp default current_timestamp,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE
/// );
pub async fn insert_transaction(
    transaction: Transaction,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the transaction
    let r = format!("0x{:x}", transaction.r);
    let s = format!("0x{:x}", transaction.s);
    let v = format!("0x{:x}", transaction.v);
    let to = format!("0x{:x}", transaction.to.unwrap_or_default());
    let gas = transaction.gas.as_u64() as i32;
    let from = format!("0x{:x}", transaction.from);
    let hash = format!("0x{:x}", transaction.hash());
    let transaction_type = transaction.transaction_type.unwrap().as_u64() as i16;
    let input = format!("{:x}", transaction.input);
    let nonce = transaction.nonce.as_u64() as i32;
    let value = Decimal::from(transaction.value.as_u128() as i64);
    let chain_id = transaction.chain_id.unwrap().as_u64().to_string();
    let gas_price = Decimal::from(transaction.gas_price.unwrap().as_u128() as i64);
    let block_hash = format!("0x{:x}", transaction.block_hash.unwrap());
    let access_list = serde_json::to_value(&transaction.access_list).unwrap();
    let block_number = transaction.block_number.unwrap().as_u64() as i64;
    let max_fee_per_gas =
        Decimal::from(transaction.max_fee_per_gas.unwrap_or_default().as_u128() as i64);
    let transaction_index = transaction.transaction_index.unwrap_or_default().as_u64() as i32;
    let max_priority_fee_per_gas = Decimal::from(
        transaction
            .max_priority_fee_per_gas
            .unwrap_or_default()
            .as_u128() as i64,
    );

    // Build the SQL query
    let query = r#"
        INSERT INTO transactions ("r", "s", "v", "to", "gas", "from", "hash", "type", "input",
                                  "nonce", "value", "chainId", "gasPrice", "blockHash",
                                  "accessList", "blockNumber", "maxFeePerGas", "transactionIndex",
                                  "maxPriorityFeePerGas")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14 ,$15, $16, $17, $18, $19)
        ON CONFLICT ("hash") DO NOTHING;
    "#;
    // Prepare the statement
    let db_client = db_pool.get().await.map_err(|e| {
        eprintln!("Error acquiring database connection: {:?}", e);
        Box::new(e) as Box<dyn Error>
    })?;
    let statement = db_client
        .prepare(query)
        .await
        .expect("Failed to prepare statement");
    // Prepare the parameter values
    let params: [&(dyn ToSql + Sync); 19] = [
        &r,
        &s,
        &v,
        &to,
        &gas,
        &from,
        &hash,
        &transaction_type,
        &input,
        &nonce,
        &value,
        &chain_id,
        &gas_price,
        &block_hash,
        &access_list,
        &block_number,
        &max_fee_per_gas,
        &transaction_index,
        &max_priority_fee_per_gas,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Transaction {} inserted successfully", hash);
            Ok(())
        }
        Err(err) => {
            eprintln!("Error inserting transaction {}: {}", hash, err);
            Err(Box::new(err))
        }
    }
}

/// Function to insert a transaction receipt into the database
/// Database schema:
/// CREATE TABLE transactions_receipts (
/// "transactionHash" VARCHAR(66) NOT NULL PRIMARY KEY,
/// "transactionIndex" INT NOT NULL,
/// "blockHash" VARCHAR(66) NOT NULL,
/// "from" VARCHAR(42) NOT NULL,
/// "to" VARCHAR(42),
/// "blockNumber" BIGINT NOT NULL,
/// "cumulativeGasUsed" INT,
/// "gasUsed" INT,
/// "contractAddress" VARCHAR(42),
/// "logs" JSON,
/// "logsBloom" TEXT,
/// "status" BOOLEAN,
/// "effectiveGasPrice" VARCHAR(78),
/// "type" VARCHAR(10),
/// "lastUpdated" timestamp default current_timestamp,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE,
/// FOREIGN KEY ("transactionHash") REFERENCES transactions("hash") ON DELETE CASCADE
/// );
pub async fn insert_transaction_receipt(
    transaction_receipt: TransactionReceipt,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the transaction
    let transaction_hash = format!("0x{:x}", transaction_receipt.transaction_hash);
    let transaction_index = transaction_receipt.transaction_index.as_u64() as i32;
    let block_hash = format!("0x{:x}", transaction_receipt.block_hash.unwrap());
    let from = format!("0x{:x}", transaction_receipt.from);
    let to = format!("0x{:x}", transaction_receipt.to.unwrap_or_default());
    let block_number = transaction_receipt.block_number.unwrap().as_u64() as i64;
    let cumulative_gas_used = Decimal::from(transaction_receipt.cumulative_gas_used.as_u128());
    let gas_used = Decimal::from(transaction_receipt.gas_used.unwrap_or_default().as_u128() as i64);
    let contract_address = format!(
        "0x{:x}",
        transaction_receipt.contract_address.unwrap_or_default()
    );
    let logs = serde_json::to_value(&transaction_receipt.logs).unwrap();
    let logs_bloom = format!("0x{:x}", transaction_receipt.logs_bloom);
    let status = if transaction_receipt.status.unwrap_or_default().as_u32() == 1 {
        true
    } else {
        false
    };
    let effective_gas_price = Decimal::from(
        transaction_receipt
            .effective_gas_price
            .unwrap_or_default()
            .as_u128(),
    );
    let transaction_type = format!("{:?}", transaction_receipt.transaction_type.unwrap());

    // Build the SQL query
    let query = r#"
        INSERT INTO transactions_receipts ("transactionHash", "transactionIndex", "blockHash", "from",
                                            "to", "blockNumber", "cumulativeGasUsed", "gasUsed",
                                            "contractAddress", "logs", "logsBloom", "status",
                                            "effectiveGasPrice", "type")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13 ,$14)
        ON CONFLICT ("transactionHash") DO NOTHING;
    "#;
    // Prepare the statement
    let db_client = db_pool.get().await.map_err(|e| {
        eprintln!("Error acquiring database connection: {:?}", e);
        Box::new(e) as Box<dyn Error>
    })?;
    let statement = db_client
        .prepare(query)
        .await
        .expect("Failed to prepare statement");
    // Prepare the parameter values
    let params: [&(dyn ToSql + Sync); 14] = [
        &transaction_hash,
        &transaction_index,
        &block_hash,
        &from,
        &to,
        &block_number,
        &cumulative_gas_used,
        &gas_used,
        &contract_address,
        &logs,
        &logs_bloom,
        &status,
        &effective_gas_price,
        &transaction_type,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Transaction receipt {} inserted successfully", transaction_hash);
            Ok(())
        }
        Err(err) => {
            eprintln!(
                "Error inserting transaction receipt {}: {}",
                transaction_hash, err
            );
            Err(Box::new(err))
        }
    }
}