// Module: db::tokens
use crate::indexer_types;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::{abi::Abi, prelude::*};
use ethers_contract::Contract;
use log::{error as log_error, debug};
use rust_decimal::prelude::*;
use std::{error::Error, sync::Arc};
use tokio_postgres::{types::ToSql, NoTls};

/// Function to insert a token into the database
/// Here we have to get the token information from the contract
/// using the contract ABI:
/// - name
/// - symbol
/// - totalSupply
/// - decimals
/// - holderCount
/// - totalSupplyUpdatedAtBlock
///
///
/// Database schema:
/// CREATE TABLE tokens (
/// "address" VARCHAR(42) NOT NULL PRIMARY KEY,
/// "type" character varying(255) NOT NULL,
/// "name" text,
/// "symbol" text,
/// "totalSupply" numeric,
/// "decimals" numeric,
/// "lastUpdated" timestamp default current_timestamp,
/// "holderCount" integer,
/// "totalSupplyUpdatedAtBlock" bigint
/// );
pub async fn insert_erc20_token(
    address: Address,
    verified_sc_data: indexer_types::ContractInfo,
    block_number: U64,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
    ws_client: Arc<Provider<Ws>>,
) -> Result<(), Box<dyn Error>> {
    debug!("Inserting ERC20 token: {}", address);
    // Get the token data using the contract ABI and ws_client
    let token_data = get_token_data(address, verified_sc_data.clone(), ws_client.clone()).await?;
    // Build the SQL query
    let query = r#"
        INSERT INTO tokens 
        ("address", "type", "name", "symbol", "totalSupply", "decimals", "holderCount", "totalSupplyUpdatedAtBlock", "insertedAt") 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW()) 
        ON CONFLICT (address) 
        DO UPDATE SET 
        "name" = EXCLUDED."name",
        "symbol" = EXCLUDED."symbol",
        "totalSupply" = EXCLUDED."totalSupply",
        "decimals" = EXCLUDED."decimals",
        "holderCount" = EXCLUDED."holderCount",
        "totalSupplyUpdatedAtBlock" = EXCLUDED."totalSupplyUpdatedAtBlock"
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
    let params: [&(dyn ToSql + Sync); 8] = [
        &address.to_string(),
        &verified_sc_data.contractType,
        &token_data.name,
        &token_data.symbol,
        &token_data.totalSupply,
        &token_data.decimals,
        &(token_data.holderCount.to_i32()),
        &(block_number.as_u64() as i64),
    ];
    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // info!("Inserted token: {}", address);
            Ok(())
        }
        Err(e) => {
            log_error!("Error inserting token: {}", address);
            log_error!("Error: {:?}", e);
            Err(Box::new(e))
        }
    }
}

async fn get_token_data(
    address: Address,
    verified_sc_data: indexer_types::ContractInfo,
    ws_client: Arc<Provider<Ws>>,
) -> Result<indexer_types::TokenInfo, Box<dyn Error>> {
    let mut token_data = indexer_types::TokenInfo::new();
    // Parse the JSON ABI
    let contract_abi: Abi =
        serde_json::from_str(verified_sc_data.abi_json.as_str().unwrap_or("[]"))
            .expect("Failed to parse ABI");
    // Create a new Contract instance
    let contract = Contract::new(address, contract_abi, ws_client);

    // Call the totalSupply function
    let total_supply: U256 = match contract.method("totalSupply", ()) {
        Ok(method) => method.call().await?,
        Err(e) => {
            log_error!("Error: {:?}", e);
            U256::zero()
        }
    };
    token_data.totalSupply = Decimal::from_str(total_supply.to_string().as_str()).unwrap();
    // Call the name function
    let name: String = match contract.method("name", ()) {
        Ok(method) => method.call().await?,
        Err(e) => {
            log_error!("Error: {:?}", e);
            String::from("")
        }
    };
    token_data.name = name;
    // Call the symbol function
    let symbol: String = match contract.method("symbol", ()) {
        Ok(method) => method.call().await?,
        Err(e) => {
            log_error!("Error: {:?}", e);
            String::from("")
        }
    };
    token_data.symbol = symbol;
    // Call the decimals function
    let decimals: U256 = match contract.method("decimals", ()) {
        Ok(method) => method.call().await?,
        Err(e) => {
            log_error!("Error: {:?}", e);
            U256::zero()
        }
    };
    token_data.decimals = Decimal::from_str(decimals.to_string().as_str()).unwrap();
    // holderCount doesn't exist in ERC20
    // TODO: Add holderCount feature

    debug!("Token data: {}", token_data.to_string());
    Ok(token_data)
}
