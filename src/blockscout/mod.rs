// Module to handle blockscout REST API requests

use crate::indexer_types;
use log::{debug, error as log_error};
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
    let (blockscout_endpoint, _blockscout_api_key, client) = connect_blockscout();
    let url = format!(
        "{}/api/v2/smart-contracts/{}", //&apikey={}
        blockscout_endpoint, address //, blockscout_api_key
    );
    let response = client.get(url).send().await.unwrap();
    // check response code is 200
    if response.status().is_client_error() {
        debug!("No verified source code found for {}", address);
        if response.status().as_u16() == 404 {
            return indexer_types::ContractInfo::new();
        }
        // print response status and returned response
        println!("Response: {:?}", response);
        return indexer_types::ContractInfo::new();
    }
    // if other than 200 and 404, log error
    if response.status().is_server_error() {
        log_error!("Error getting verified source code for {}", address);
        println!("Response: {:?}", response);
        return indexer_types::ContractInfo::new();
    }
    // Deserialize the JSON response into the ContractInfo struct
    // let json = response.json::<Value>().await.unwrap();
    let json = match response.json::<Value>().await {
        Ok(result) => {
            // println!("Parsed JSON: {:?}", result);
            result
        }
        Err(e) => {
            log_error!("Error parsing JSON");
            log_error!("Error: {}", e);
            serde_json::from_value(serde_json::json!([])).unwrap()
        }
    };
    // check if json has result field and if it is not empty
    if json.is_null() {
        debug!("Error smart contract JSON is null");
        return indexer_types::ContractInfo::new();
    }
    // Serialize the ContractInfo struct with specific field names
    let res = indexer_types::ContractInfo {
        contractType: indexer_types::ContractType::detect_contract_type(json["abi"].clone())
            .to_string(),
        abi_json: json["abi"].clone(),
        abi: json["abi"].clone().to_string(),
        additionalSources: json["additional_sources"].clone().to_string(),
        compilerSettings: json["compiler_settings"].clone().to_string(),
        compilerVersion: json["compiler_version"].clone().to_string(),
        constructorArguments: json["constructor_args"].clone().to_string(),
        contractName: json["name"].clone().to_string(),
        EVMVersion: json["evm_version"].clone().to_string(),
        fileName: json["file_path"].clone().to_string(),
        isProxy: false, //json["IsProxy"].clone().to_string() == "true",
        optimizationUsed: json["optimization_enabled"].clone().to_string() == "true",
        sourceCode: json["source_code"].clone().to_string(),
    };

    debug!("Got verified source code for {}", address);
    res
}
