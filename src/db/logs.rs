// Module: db::logs
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::{abi::Abi, prelude::*};
use ethers_contract::Contract;
use log::{debug, error as log_error};
use rust_decimal::prelude::*;
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
    let log_type = log.log_type;
    let first_topic = format!("0x{:x}", log.topics[0]);
    let second_topic = format!("0x{:x}", log.topics[1]);
    let third_topic = format!("0x{:x}", log.topics[2]);
    let fourth_topic =  format!("0x{:x}", log.topics[3]);
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
        log_error!("Error acquiring database connection: {:?}", e);
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
            // info!("Inserted log: {}", address);
            Ok(())
        }
        Err(e) => {
            log_error!("Error inserting log: {}", address);
            log_error!("Error: {:?}", e);
            Err(Box::new(e))
        }
    }


    //TODO: Detect token transfert in logs and store token transfert in DB
    // 1- Try to get the ABI from the address
    // 2- Find the event in the ABI using the log's topic[0] (event signature hash)
    // 3- If token type is ERC20 and event name is "Transfer", then we have a token transfert
}
