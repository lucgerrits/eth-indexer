// Module: db::contracts

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use serde_json;
use std::error::Error;
use tokio_postgres::{types::ToSql, NoTls};


/// Function to insert smart contract information into the database
/// Database schema:
/// CREATE TABLE contracts (
/// "address" VARCHAR(42) NOT NULL PRIMARY KEY,
/// "blockNumber" BIGINT NOT NULL,
/// "transactionHash" VARCHAR(66) NOT NULL,
/// "creatorAddress" VARCHAR(42) NOT NULL,
/// "contractType" character varying(255),
/// "abi" JSON,
/// "sourceCode" TEXT,
/// "additionalSources" TEXT,
/// "compilerSettings" TEXT,
/// "constructorArguments" TEXT,
/// "EVMVersion" TEXT,
/// "fileName" TEXT,
/// "isProxy" BOOLEAN,
/// "contractName" TEXT,
/// "compilerVersion" TEXT,
/// "optimizationUsed" BOOLEAN,
/// "bytecode" TEXT,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE,
/// FOREIGN KEY ("transactionHash") REFERENCES transactions("hash") ON DELETE CASCADE
/// );
pub async fn insert_smart_contract(
    transaction_receipt: TransactionReceipt,
    code: Bytes,
    verified_source_code: serde_json::Value,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // It is possible that the verified_source_code is empty
    // We will insert the minimum information into the database

    // Extract relevant data from the transaction receipt
    // if verified_source_code is null or empty, we will insert the minimum information
    let address = format!("0x{:x}", transaction_receipt.contract_address.unwrap());
    let block_number = transaction_receipt.block_number.unwrap().as_u64() as i64;
    let transaction_hash = format!("0x{:x}", transaction_receipt.transaction_hash);
    let creator_address = format!("0x{:x}", transaction_receipt.from);
    let contract_type = if verified_source_code.is_null() {
        String::from("Unknown")
    } else {
        verified_source_code["contractType"].to_string()
    };
    let abi = if verified_source_code.is_null() {
        serde_json::Value::Null
    } else {
        verified_source_code["abi"].clone()
    };
    let source_code = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["sourceCode"].to_string()
    };
    let additional_sources = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["additionalSources"].to_string()
    };
    let compiler_settings = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["compilerSettings"].to_string()
    };
    let constructor_arguments = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["constructorArguments"].to_string()
    };
    let evm_version = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["EVMVersion"].to_string()
    };
    let file_name = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["fileName"].to_string()
    };
    let is_proxy = if verified_source_code.is_null() {
        false
    } else {
        verified_source_code["isProxy"]
            .as_bool()
            .unwrap_or_default()
    };
    let contract_name = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["contractName"].to_string()
    };
    let compiler_version = if verified_source_code.is_null() {
        String::from("")
    } else {
        verified_source_code["compilerVersion"].to_string()
    };
    let optimization_used = if verified_source_code.is_null() {
        false
    } else {
        verified_source_code["optimizationUsed"]
            .as_bool()
            .unwrap_or_default()
    };
    let bytecode = format!("{:x}", code);

    // Build the SQL query
    let query = r#"
            INSERT INTO contracts 
                ("address", "bytecode", "blockNumber", "transactionHash", "creatorAddress",
                "contractType",
                "abi", "sourceCode", "additionalSources", "compilerSettings",
                "constructorArguments", "EVMVersion", "fileName", "isProxy",
                "contractName", "compilerVersion", "optimizationUsed")
                VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT ("address")
            DO UPDATE SET
                "bytecode" = EXCLUDED."bytecode",
                "blockNumber" = EXCLUDED."blockNumber",
                "transactionHash" = EXCLUDED."transactionHash",
                "creatorAddress" = EXCLUDED."creatorAddress",
                "contractType" = EXCLUDED."contractType",
                "abi" = EXCLUDED."abi",
                "sourceCode" = EXCLUDED."sourceCode",
                "additionalSources" = EXCLUDED."additionalSources",
                "compilerSettings" = EXCLUDED."compilerSettings",
                "constructorArguments" = EXCLUDED."constructorArguments",
                "EVMVersion" = EXCLUDED."EVMVersion",
                "fileName" = EXCLUDED."fileName",
                "isProxy" = EXCLUDED."isProxy",
                "contractName" = EXCLUDED."contractName",
                "compilerVersion" = EXCLUDED."compilerVersion",
                "optimizationUsed" = EXCLUDED."optimizationUsed"
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
    let params: [&(dyn ToSql + Sync); 17] = [
        &address,
        &bytecode,
        &block_number,
        &transaction_hash,
        &creator_address,
        &contract_type,
        &abi,
        &source_code,
        &additional_sources,
        &compiler_settings,
        &constructor_arguments,
        &evm_version,
        &file_name,
        &is_proxy,
        &contract_name,
        &compiler_version,
        &optimization_used,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Smart contract {} inserted/updated successfully", address);
            Ok(())
        }
        Err(err) => {
            eprintln!(
                "Error inserting/updating smart contract {}: {}",
                address, err
            );
            Err(Box::new(err))
        }
    }
}
