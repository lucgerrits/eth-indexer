// Module: db::contracts

use crate::{
    db::{logs, tokens},
    indexer_types,
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use log::{debug, warn, error as log_error};
use serde_json;
use std::{error::Error, sync::Arc};
use tokio_postgres::{types::ToSql, NoTls};

/// Function to insert smart contract information into the database
/// Particularity is that we need the ws_client to get the smart contract data if we have the ABI.
///
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
    verified_sc_data: indexer_types::ContractInfo,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
    ws_client: Arc<Provider<Ws>>,
) -> Result<(), Box<dyn Error>> {
    // It is possible that the verified_source_code is empty
    // We will insert the minimum information into the database

    // Extract relevant data from the transaction receipt
    // if verified_source_code is null or empty, we will insert the minimum information
    let address = format!("0x{:x}", transaction_receipt.contract_address.unwrap());
    let block_number = transaction_receipt.block_number.unwrap().as_u64() as i64;
    let transaction_hash = format!("0x{:x}", transaction_receipt.transaction_hash);
    let creator_address = format!("0x{:x}", transaction_receipt.from);
    let abi: serde_json::Value = if verified_sc_data.is_null() {
        serde_json::from_str("[]").unwrap()
    } else {
        serde_json::from_str(verified_sc_data.clone().abi.as_str()).unwrap()
    };
    let contract_type = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().contractType
    };
    let source_code = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().sourceCode
    };
    let additional_sources = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().additionalSources
    };
    let compiler_settings = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().compilerSettings
    };
    let constructor_arguments = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().constructorArguments
    };
    let evm_version = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().EVMVersion
    };
    let file_name = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().fileName
    };
    let is_proxy = if verified_sc_data.is_null() {
        false
    } else {
        verified_sc_data.clone().isProxy
    };
    let contract_name = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().contractName
    };
    let compiler_version = if verified_sc_data.is_null() {
        String::from("")
    } else {
        verified_sc_data.clone().compilerVersion
    };
    let optimization_used = if verified_sc_data.is_null() {
        false
    } else {
        verified_sc_data.clone().optimizationUsed
    };
    let bytecode = format!("{:x}", code);

    // Build the SQL query
    let query = r#"
            INSERT INTO contracts 
                ("address", "bytecode", "blockNumber", "transactionHash", "creatorAddress",
                "contractType",
                "abi", "sourceCode", "additionalSources", "compilerSettings",
                "constructorArguments", "EVMVersion", "fileName", "isProxy",
                "contractName", "compilerVersion", "optimizationUsed", "insertedAt")
                VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW())
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
        log_error!("Error acquiring database connection: {}", e);
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
            debug!("Smart contract {} inserted/updated successfully", address);
            if !verified_sc_data.is_null() {
                debug!("Detected contract type: {}", contract_type);
                if contract_type == "ERC20" {
                    tokens::insert_erc20_token(
                        transaction_receipt.contract_address.unwrap(),
                        verified_sc_data.clone(),
                        transaction_receipt.block_number.unwrap(),
                        db_pool.clone(),
                        ws_client.clone(),
                    )
                    .await?;

                    //trick to process logs from constructor, that seems to not show up in the receipts logs
                    let filter_by_block =
                        Filter::new().from_block(transaction_receipt.block_number.unwrap());
                    let constructor_logs: Vec<Log> = match  ws_client.get_logs(&filter_by_block).await {
                        Ok(logs) => logs,
                        Err(err) => {
                            log_error!("Error getting logs for contract 0x{:x}: {}", transaction_receipt.contract_address.unwrap(), err);
                            vec![]
                        }
                    };

                    for log in constructor_logs {
                        logs::insert_log(log, db_pool.clone(), ws_client.clone()).await?;
                    }
                } else {
                    //TODO: Handle other contract types
                    warn!("Contract type '{}' is not supported yet", contract_type.to_string());
                }
            }
            Ok(())
        }
        Err(err) => {
            log_error!(
                "Error inserting/updating smart contract {}: {}",
                address,
                err
            );
            Err(Box::new(err))
        }
    }
}
