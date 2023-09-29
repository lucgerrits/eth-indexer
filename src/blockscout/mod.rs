// Module to handle blockscout REST API requests

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
    println!("Connected to blockscout endpoint!");
    (blockscout_endpoint, blockscout_api_key, client)
}

/// Function to get the verified source code of a contract
/// Returns a JSON object
pub async fn get_verified_source_code(address: String) -> Value {
    let (blockscout_endpoint, blockscout_api_key, client) = connect_blockscout();
    let url = format!(
        "{}/api?module=contract&action=getsourcecode&address={}&apikey={}",
        blockscout_endpoint, address, blockscout_api_key
    );
    let response = client.get(url).send().await.unwrap();
    // Deserialize the JSON response into the ContractInfo struct
    let json = response.json::<Value>().await.unwrap();

    // check if json has result field and if it is not empty
    if json["result"].is_null() || json["result"].as_array().unwrap().is_empty() {
        println!("No verified source code found for {}", address);
        return serde_json::Value::Null;
    }

    // Serialize the ContractInfo struct with specific field names
    let res = serde_json::json!({
        "abi": json["result"][0]["ABI"],
        "additionalSources": json["result"][0]["AdditionalSources"],
        "compilerSettings": json["result"][0]["CompilerSettings"],
        "compilerVersion":  json["result"][0]["CompilerVersion"],
        "constructorArguments": json["result"][0]["ConstructorArguments"],
        "contractName": json["result"][0]["ContractName"],
        "EVMVersion": json["result"][0]["EVMVersion"],
        "fileName": json["result"][0]["FileName"],
        "isProxy": json["result"][0]["IsProxy"],
        "optimizationUsed": json["result"][0]["OptimizationUsed"],
        "sourceCode": json["result"][0]["SourceCode"],
    });

    println!("Got verified source code for {}", address);
    // println!("JSON: {}", res);

    res
}
