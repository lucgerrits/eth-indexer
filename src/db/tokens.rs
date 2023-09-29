// Module: db::tokens
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use rust_decimal::prelude::*;
use std::error::Error;
use tokio_postgres::{types::ToSql, NoTls};

// ContractType is an enum that represents the type of a smart contract
pub enum ContractType {
    Unknown,
    ERC20,
    ERC721,
    ERC777,
    ERC1155,
}
impl ContractType {
    pub fn to_string(&self) -> String {
        match self {
            ContractType::Unknown => String::from(""),
            ContractType::ERC20 => String::from("ERC20"),
            ContractType::ERC721 => String::from("ERC721"),
            ContractType::ERC777 => String::from("ERC777"),
            ContractType::ERC1155 => String::from("ERC1155"),
        }
    }
}

/// Function to insert a token into the database
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
    abi_json: serde_json::Value,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the token
    let address = format!("0x{:x}", address);
    let token_type = detect_contract_type(&abi_json).to_string();
    let name = abi_json["name"].to_string();
    let symbol = abi_json["symbol"].to_string();
    let total_supply = Decimal::from_str(&abi_json["totalSupply"].to_string()).unwrap_or_default();
    let decimals = Decimal::from_str(&abi_json["decimals"].to_string()).unwrap_or_default();
    let holder_count = abi_json["holderCount"].as_i64().unwrap_or_default();
    let total_supply_updated_at_block = abi_json["totalSupplyUpdatedAtBlock"]
        .as_i64()
        .unwrap_or_default();

    // Build the SQL query
    let query = r#"
        INSERT INTO tokens ("address", "type", "name", "symbol", "totalSupply", "decimals",
                            "holderCount", "totalSupplyUpdatedAtBlock")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT ("address") DO UPDATE
        SET "type" = $2,
            "name" = $3,
            "symbol" = $4,
            "totalSupply" = $5,
            "decimals" = $6,
            "holderCount" = $7,
            "totalSupplyUpdatedAtBlock" = $8,
            "lastUpdated" = current_timestamp
        WHERE tokens."address" = $1
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

    // Prepare the parameters
    let params: [&(dyn ToSql + Sync); 8] = [
        &address,
        &token_type,
        &name,
        &symbol,
        &total_supply,
        &decimals,
        &holder_count,
        &total_supply_updated_at_block,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Inserted token {} into database", address);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error inserting token into database: {:?}", e);
            Err(Box::new(e))
        }
    }
}

// Helper function to check if a specific function is in the ABI
fn has_function(abi: &serde_json::Value, function_name: &str) -> bool {
    if let Some(array) = abi.as_array() {
        for item in array {
            if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                if let Some(typ) = item.get("type").and_then(|t| t.as_str()) {
                    if name == function_name && typ == "function" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// Main function to detect the contract type
// pub fn detect_contract_type(abi_json: &serde_json::Value) -> ContractType {
//     if abi_json.is_null() {
//         return ContractType::Unknown;
//     }

//     // Check if the contract is an ERC20 token
//     if has_function(abi_json, "totalSupply")
//         && has_function(abi_json, "balanceOf")
//         && has_function(abi_json, "transfer")
//     {
//         println!("ERC20 token detected");
//         return ContractType::ERC20;
//     }
//     // Check if the contract is an ERC721 token
//     else if has_function(abi_json, "ownerOf")
//         && has_function(abi_json, "safeTransferFrom")
//         && has_function(abi_json, "transferFrom")
//     {
//         println!("ERC721 token detected");
//         return ContractType::ERC721;
//     }
//     // Check if the contract is an ERC777 token
//     else if has_function(abi_json, "granularity")
//         && has_function(abi_json, "defaultOperators")
//         && has_function(abi_json, "send")
//     {
//         println!("ERC777 token detected");
//         return ContractType::ERC777;
//     }
//     // Check if the contract is an ERC1155 token
//     else if has_function(abi_json, "safeTransferFrom")
//         && has_function(abi_json, "safeBatchTransferFrom")
//         && has_function(abi_json, "balanceOf")
//         && has_function(abi_json, "balanceOfBatch")
//     {
//         println!("ERC1155 token detected");
//         return ContractType::ERC1155;
//     }

//     println!("Unknown token detected");
//     ContractType::Unknown
// }


use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, Serialize)]
struct ABIEntry {
    name: String,
    inputs: Option<Vec<ABIParam>>,
    outputs: Option<Vec<ABIParam>>,
    r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ABIParam {
    name: String,
    internalType: Option<String>,
    r#type: String,
}

pub fn detect_contract_type(abi_json: &serde_json::Value) -> ContractType {
    let parsed_abi: Vec<ABIEntry> = match serde_json::from_str(abi_json.to_string().as_str()) {
        Ok(result) => result,
        Err(_) => serde_json::from_value(json!([])).unwrap(),
    };

    // Check for ERC-20 functions
    let erc20_functions = vec!["totalSupply", "balanceOf", "transfer"];
    let erc721_events = vec!["Transfer"];

    for entry in parsed_abi.iter() {
        if entry.r#type == "function" && erc20_functions.contains(&&*entry.name) {
            return ContractType::ERC20;
        } else if entry.r#type == "event" && erc721_events.contains(&&*entry.name) {
            return ContractType::ERC721;
        }
    }

    // Add more checks for other token types if needed

    ContractType::Unknown
}