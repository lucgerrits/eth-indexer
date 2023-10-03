// Module to handle blockscout REST API requests

use crate::indexer_types;
use log::{error as log_error, debug};
use reqwest::Client;
use serde_json::Value;
use std::env;

/// Function to connect to the blockscout REST API endpoint
/// Returns a client
fn connect_blockscout() -> (String, String, Client) {
    let blockscout_endpoint = env::var("BLOCKSCOUT_ENDPOINT").unwrap();
    let blockscout_api_key = env::var("BLOCKSCOUT_API_KEY").unwrap();
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    // println!("Connected to blockscout endpoint!");
    (blockscout_endpoint, blockscout_api_key, client)
}

/// Function to get the verified data of a contract
/// Returns a JSON object
pub async fn get_verified_sc_data(address: String) -> indexer_types::ContractInfo {
    let (blockscout_endpoint, blockscout_api_key, client) = connect_blockscout();
    let url = format!(
        "{}/api?module=contract&action=getsourcecode&address={}&apikey={}",
        blockscout_endpoint, address, blockscout_api_key
    );
    let response = client.get(url).send().await.unwrap();
    // Deserialize the JSON response into the ContractInfo struct
    // let json = response.json::<Value>().await.unwrap();
    let json = match response.json::<Value>().await {
        Ok(result) => {
            // println!("Parsed JSON: {:?}", result);
            result
        }
        Err(e) => {
            log_error!("Error parsing JSON");
            log_error!("Error: {:?}", e);
            serde_json::from_value(serde_json::json!([])).unwrap()
        }
    };

    // check if json has result field and if it is not empty
    if json["result"].is_null() || json["result"].as_array().unwrap().is_empty() {
        debug!("No verified source code found for {}", address);
        return serde_json::from_str("{}").unwrap();
    }
    // Serialize the ContractInfo struct with specific field names
    let res = indexer_types::ContractInfo {
        contractType: indexer_types::ContractType::detect_contract_type(
            json["result"][0]["ABI"].clone(),
        ).to_string(),
        abi_json : json["result"][0]["ABI"].clone(),
        abi: json["result"][0]["ABI"].clone().to_string(),
        additionalSources: json["result"][0]["AdditionalSources"].clone().to_string(),
        compilerSettings: json["result"][0]["CompilerSettings"].clone().to_string(),
        compilerVersion: json["result"][0]["CompilerVersion"].clone().to_string(),
        constructorArguments: json["result"][0]["ConstructorArguments"]
            .clone()
            .to_string(),
        contractName: json["result"][0]["ContractName"].clone().to_string(),
        EVMVersion: json["result"][0]["EVMVersion"].clone().to_string(),
        fileName: json["result"][0]["FileName"].clone().to_string(),
        isProxy: json["result"][0]["IsProxy"].clone().to_string() == "true",
        optimizationUsed: json["result"][0]["OptimizationUsed"].clone().to_string() == "true",
        sourceCode: json["result"][0]["SourceCode"].clone().to_string(),
    };
   
    debug!("Got verified source code for {}", address);
    res
}
