// Module: indexer_types
use ethers::prelude::*;
use ethers_contract::{EthAbiCodec, EthAbiType};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

// ContractType is an enum that represents the type of a smart contract
pub enum ContractType {
    Unknown,
    ERC20,
    ERC721,
    // ERC777,
    // ERC1155,
}
impl ContractType {
    pub fn to_string(&self) -> String {
        match self {
            ContractType::Unknown => String::from(""),
            ContractType::ERC20 => String::from("ERC20"),
            ContractType::ERC721 => String::from("ERC721"),
            // ContractType::ERC777 => String::from("ERC777"),
            // ContractType::ERC1155 => String::from("ERC1155"),
        }
    }
    pub fn detect_contract_type(abi_json: serde_json::Value) -> ContractType {
        let mut contract_type = ContractType::Unknown;
        if abi_json.is_null() {
            return contract_type;
        }

        let abi_str = abi_json.as_str().expect("ABI is not a string");
        let parsed_abi: serde_json::Value =
            serde_json::from_str(&abi_str).expect("Failed to parse ABI JSON");

        // Check for ERC-20 functions
        let erc20_functions = vec!["totalSupply", "balanceOf", "transfer"];
        let erc721_events = vec!["Transfer"];

        fn all_names_found(parsed_abi: &serde_json::Value, names_to_check: &[&str]) -> bool {
            let mut found_names = Vec::new();

            // Iterate over the array and check "name" fields
            if let Some(abi_array) = parsed_abi.as_array() {
                for abi_object in abi_array {
                    if let Some(obj_type) = abi_object["type"].as_str() {
                        if obj_type == "function" {
                            if let Some(name) = abi_object["name"].as_str() {
                                found_names.push(name);
                            }
                        }
                    }
                }
            }

            // Check if all names_to_check are found in found_names
            names_to_check
                .iter()
                .all(|&name| found_names.contains(&name))
        }

        // Check if all ERC-20 functions are found
        if all_names_found(&parsed_abi, &erc20_functions) {
            contract_type = ContractType::ERC20;
        }
        // Check if all ERC-721 events are found
        else if all_names_found(&parsed_abi, &erc721_events) {
            contract_type = ContractType::ERC721;
        }

        contract_type
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct ContractInfo {
    pub contractType: String,
    pub abi: String,
    pub abi_json: serde_json::Value,
    pub additionalSources: String,
    pub compilerSettings: String,
    pub compilerVersion: String,
    pub constructorArguments: String,
    pub contractName: String,
    pub EVMVersion: String,
    pub fileName: String,
    pub isProxy: bool,
    pub optimizationUsed: bool,
    pub sourceCode: String,
}
impl ContractInfo {
    pub fn new() -> Self {
        ContractInfo {
            contractType: String::from(""),
            abi: String::from(""),
            abi_json: serde_json::from_str("[]").unwrap(),
            additionalSources: String::from(""),
            compilerSettings: String::from(""),
            compilerVersion: String::from(""),
            constructorArguments: String::from(""),
            contractName: String::from(""),
            EVMVersion: String::from(""),
            fileName: String::from(""),
            isProxy: false,
            optimizationUsed: false,
            sourceCode: String::from(""),
        }
    }
    pub fn is_null(&self) -> bool {
        self.contractType.is_empty()
            && self.abi.is_empty()
            && self.abi_json.is_null()
            && self.additionalSources.is_empty()
            && self.compilerSettings.is_empty()
            && self.compilerVersion.is_empty()
            && self.constructorArguments.is_empty()
            && self.contractName.is_empty()
            && self.EVMVersion.is_empty()
            && self.fileName.is_empty()
            && !self.isProxy
            && !self.optimizationUsed
            && self.sourceCode.is_empty()
    }
}
impl fmt::Display for ContractInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\n\tContract Type: {}\n\tABI: {}\n\tABI JSON: {}\n\tAdditional Sources: {}\n\tCompiler Settings: {}\n\tCompiler Version: {}\n\tConstructor Arguments: {}\n\tContract Name: {}\n\tEVM Version: {}\n\tFile Name: {}\n\tIs Proxy: {}\n\tOptimization Used: {}\n\tSource Code: {}",
            self.contractType,
            self.abi,
            self.abi_json,
            self.additionalSources,
            self.compilerSettings,
            self.compilerVersion,
            self.constructorArguments,
            self.contractName,
            self.EVMVersion,
            self.fileName,
            self.isProxy,
            self.optimizationUsed,
            self.sourceCode,
        )
    }
}

#[allow(non_snake_case)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub totalSupply: Decimal,
    pub decimals: Decimal,
    pub holderCount: Decimal,
    pub totalSupplyUpdatedAtBlock: String,
}

impl TokenInfo {
    pub fn new() -> Self {
        TokenInfo {
            name: String::from(""),
            symbol: String::from(""),
            totalSupply: Decimal::zero(),
            decimals: Decimal::zero(),
            holderCount: Decimal::zero(),
            totalSupplyUpdatedAtBlock: String::from(""),
        }
    }
    pub fn is_null(&self) -> bool {
        self.name.is_empty()
            && self.symbol.is_empty()
            && self.totalSupply.is_zero()
            && self.decimals.is_zero()
            && self.holderCount.is_zero()
            && self.totalSupplyUpdatedAtBlock.is_empty()
    }
    pub fn to_string(&self) -> String {
        format!(
            "\n\tName: {}\n\tSymbol: {}\n\tTotal Supply: {}\n\tDecimals: {}\n\tHolder Count: {}\n\tTotal Supply Updated At Block: {}",
            self.name,
            self.symbol,
            self.totalSupply,
            self.decimals,
            self.holderCount,
            self.totalSupplyUpdatedAtBlock,
        )
    }
}

// ERC20 event Transfer(address,address,uint256)
#[derive(Debug, Clone, EthAbiType, EthAbiCodec)]
pub struct Transfert {
    pub from: Address,
    pub to: Address,
    pub value: U256,
}
