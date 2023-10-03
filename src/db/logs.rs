// Module: db::logs
use crate::db::{self, tokens};
use crate::indexer_types;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::utils::keccak256;
use ethers::{abi::Abi, prelude::*};
use ethers_contract::Contract;
use log::{debug, error as log_error};
use std::{error::Error, sync::Arc};
use tokio_postgres::{types::ToSql, NoTls};

/// Function to insert a log into the database
/// Database schema:
/// CREATE TABLE logs (
///     "data"Bytea,
///     "index" integer,
///     "type" VARCHAR(255),
///     "firstTopic" VARCHAR(255),
///     "secondTopic" VARCHAR(255),
///     "thirdTopic" VARCHAR(255),
///     "fourthTopic" VARCHAR(255),
///     "address" VARCHAR(42) NOT NULL,
///     "transactionHash" VARCHAR(66) NOT NULL,
///     "blockHash" VARCHAR(66) NOT NULL,
///     "blockNumber" BIGINT NOT NULL,
///     "insertedAt" timestamp,
///     "updatedAt" timestamp default current_timestamp,
///     CONSTRAINT logs_pkey PRIMARY KEY ("transactionHash", "blockHash", "index")
/// );
pub async fn insert_log(
    log: Log,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
    _ws_client: Arc<Provider<Ws>>,
) -> Result<(), Box<dyn Error>> {
    debug!("Inserting log: {:?}", log.address.to_string());

    // Extract relevant data from the log
    let data = log.data.to_vec();
    let index = log.log_index.unwrap().as_u64() as i32;
    let log_type = log.clone().log_type;
    let first_topic = match log.topics.get(0) {
        Some(topic) => format!("0x{:x}", topic),
        None => "".to_string(),
    };
    let second_topic = match log.topics.get(1) {
        Some(topic) => format!("0x{:x}", topic),
        None => "".to_string(),
    };
    let third_topic = match log.topics.get(2) {
        Some(topic) => format!("0x{:x}", topic),
        None => "".to_string(),
    };
    let fourth_topic = match log.topics.get(3) {
        Some(topic) => format!("0x{:x}", topic),
        None => "".to_string(),
    };
    let address = format!("0x{:x}", log.address);
    let transaction_hash = format!("0x{:x}", log.transaction_hash.unwrap());
    let block_hash = format!("0x{:x}", log.block_hash.unwrap());
    let block_number = log.block_number.unwrap().as_u64() as i64;

    // Build the SQL query
    let query = r#"
        INSERT INTO logs 
        ("data", "index", "type", "firstTopic", "secondTopic", "thirdTopic", "fourthTopic", "address", "transactionHash", "blockHash", "blockNumber", "insertedAt") 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW()) 
        ON CONFLICT ("transactionHash", "blockHash", "index") 
        DO UPDATE SET 
        "data" = EXCLUDED."data",
        "type" = EXCLUDED."type",
        "firstTopic" = EXCLUDED."firstTopic",
        "secondTopic" = EXCLUDED."secondTopic",
        "thirdTopic" = EXCLUDED."thirdTopic",
        "fourthTopic" = EXCLUDED."fourthTopic",
        "address" = EXCLUDED."address",
        "blockNumber" = EXCLUDED."blockNumber"
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
    let params: [&(dyn ToSql + Sync); 11] = [
        &data,
        &index,
        &log_type,
        &first_topic,
        &second_topic,
        &third_topic,
        &fourth_topic,
        &address,
        &transaction_hash,
        &block_hash,
        &block_number,
    ];
    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            debug!("Inserted log: {}", address);
        }
        Err(e) => {
            log_error!("Error inserting log: {}", address);
            log_error!("Error: {}", e);
            return Err(Box::new(e));
        }
    }

    // From here on: Detect token transfert in logs and store token transfert in DB

    // Get the ABI for the contract address
    let abi: serde_json::Value =
        match db::get_abi_by_address(address.clone(), db_pool.clone()).await {
            Ok(abi) => abi,
            Err(e) => {
                // if error is "abi is null" then return ok
                if e.to_string() == "abi is null" {
                    return Ok(());
                }
                return Err(e);
            }
        };
    debug!("ABI found for address: {}", address);

    // Parse the JSON ABI
    let contract_abi: Abi =
        serde_json::from_str(abi.as_str().unwrap_or("[]")).expect("Failed to parse ABI");
    let contract = Contract::new(
        log.clone().address.clone(),
        contract_abi,
        _ws_client.clone(),
    );
    let contract_type = indexer_types::ContractType::detect_contract_type(abi.clone());

    match contract_type {
        indexer_types::ContractType::ERC20 => {
            // Decode the log data
            let decoded_log: indexer_types::Transfert = contract
                .decode_event("Transfer", log.clone().topics, log.clone().data)
                .unwrap();
            debug!("Decoded log: {:?}", decoded_log);

            // Compute the hash of the "Transfert" event signature.
            let transfert_signature_hash =
                H256::from(keccak256("Transfer(address,address,uint256)".as_bytes()));

            // Check if the log is a Transfert event
            if let Some(topic) = log.clone().topics.get(0) {
                if *topic == transfert_signature_hash {
                    debug!("Found Transfert {} at block: {}", address, block_number);
                    // Store the transfert in the database
                    match tokens::insert_erc20_transfert(log.clone(), decoded_log.clone(), db_pool.clone()).await {
                        Ok(_) => {
                            debug!("Transfert inserted successfully");
                            return Ok(());
                        }
                        Err(e) => {
                            log_error!("Error inserting Transfert: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
        }
        _ => {
            //TODO: Handle other contract types
            debug!("Contract type is not supported yet");
        }
    }
    Ok(())
}
