// Module to handle postgress database
// db/mod.rs
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use ethers::prelude::*;
use rust_decimal::prelude::*;
use serde_json;
use std::env;
use std::error::Error;
use std::fs;
use tokio_postgres::{types::ToSql, Client as PostgresClient, NoTls};
/// Function to connect to the postgress database
pub async fn connect_db() -> Pool<PostgresConnectionManager<NoTls>> {
    let database = env::var("POSTGRES_DB").unwrap();
    let host = env::var("POSTGRES_HOST").unwrap();
    let user = env::var("POSTGRES_USER").unwrap();
    let password = env::var("POSTGRES_PASSWORD").unwrap();
    let port = env::var("POSTGRES_PORT").unwrap();
    let url: String = format!(
        "host={} port={} user={} password={}",
        host, port, user, password
    );
    let url_with_db: String = format!("{} dbname={}", url, database);
    // Check if the database exists
    let database_exists = check_database_exists(&url, &database).await;

    if !database_exists {
        // If the database does not exist, create it
        create_database(&host, &port, &user, &password, &database, &url)
            .await
            .expect("Failed to create database");
    }

    let manager = PostgresConnectionManager::new_from_stringlike(url_with_db, NoTls)
        .expect("Failed to create connection manager");

    let pool = Pool::builder()
        .build(manager)
        .await
        .expect("Failed to create connection pool");

    println!("Connected to database!");
    pool
}

async fn check_database_exists(url: &str, database_name: &str) -> bool {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("Failed to connect to the database for checking existence");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("database connection error for existence check: {}", e);
        }
    });

    let rows = client
        .query(
            "SELECT 1 FROM pg_database WHERE datname = $1",
            &[&database_name],
        )
        .await
        .expect("Failed to check database existence");

    !rows.is_empty()
}

pub async fn create_database(
    host: &str,
    port: &str,
    user: &str,
    password: &str,
    database: &str,
    url: &str,
) -> Result<PostgresClient, tokio_postgres::Error> {
    println!(
        "Database \"{}\" does not exist. Creating database...",
        database
    );

    // Connect to the default database (e.g., "postgres") first
    let default_url = format!(
        "host={} port={} user={} password={}",
        host, port, user, password
    );
    let (client, connection) = tokio_postgres::connect(&default_url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Default database connection error: {}", e);
        }
    });

    // Create the database
    client
        .execute(&format!("CREATE DATABASE \"{}\"", database), &[])
        .await?;

    // Connect to the newly created database
    let (client, connection) = tokio_postgres::connect(url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("New database connection error: {}", e);
        }
    });

    Ok(client)
}

/// Function to initialize the database
///
/// It will check if the configuration table exists and if the version matches
/// the environment variable. If not, it will execute the SQL files in the
/// order specified by the environment variable POSTGRES_CREATE_TABLE_ORDER.
/// It will also update the version in the configuration table.
///
/// If the configuration table does not exist, it will execute the SQL files
/// in the order specified by the environment variable POSTGRES_CREATE_TABLE_ORDER
/// and create the configuration table with the version specified by the
/// environment variable VERSION.
///
/// If the configuration table exists but the version does not match, it will
/// execute the SQL files in the order specified by the environment variable
/// POSTGRES_CREATE_TABLE_ORDER and update the version in the configuration
/// table with the version specified by the environment variable VERSION.
///
pub async fn init_db(
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    let db_client = db_pool.get().await?;
    let config_version = env::var("VERSION").unwrap_or_default();
    let config_name = "version";

    // Check if the configuration table exists
    let table_exists = check_table_exists(&db_client, "configuration").await;

    if table_exists {
        // Check if the version in the configuration matches the environment variable
        let version_query = format!(
            "SELECT config_value FROM configuration WHERE config_name = '{}'",
            config_name
        );

        if let Ok(row) = db_client.query_one(&version_query, &[]).await {
            let stored_version: &str = row.try_get("config_value").unwrap_or_default();

            if stored_version == config_version {
                println!("Database is up-to-date. Skipping initialization.");
                return Ok(());
            }
        }
    }

    // If the table doesn't exist or the versions don't match, perform initialization
    let sql_files = env::var("POSTGRES_CREATE_TABLE_ORDER").unwrap();
    let sql_file_paths: Vec<&str> = sql_files.split(",").collect();
    for sql_file_path in sql_file_paths {
        let full_sql_file_path = format!("./model/{}.sql", sql_file_path);
        println!("executing sql file: {}", full_sql_file_path);
        let sql = match fs::read_to_string(&full_sql_file_path) {
            Ok(sql) => sql,
            Err(e) => panic!("Error reading sql file: {}", e),
        };

        // Execute SQL queries using prepared statements
        if let Err(e) = db_client.batch_execute(&sql).await {
            eprintln!("Error executing SQL from {}: {:?}", sql_file_path, e);
        } else {
            println!("Executed SQL from: {:?}", sql_file_path);
        }
    }

    // Update the version configuration
    let update_version_query = format!(
        "INSERT INTO configuration (config_name, config_value) VALUES ('{}', '{}')
         ON CONFLICT (config_name) DO UPDATE SET config_value = EXCLUDED.config_value",
        config_name, config_version
    );

    if let Err(e) = db_client.batch_execute(&update_version_query).await {
        eprintln!("Error updating version in configuration: {:?}", e);
    }

    Ok(())
}

// Helper function to check if a table exists
async fn check_table_exists(client: &PostgresClient, table_name: &str) -> bool {
    let query = format!(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_name = '{}'
        )",
        table_name
    );

    if let Ok(row) = client.query_one(&query, &[]).await {
        let exists: bool = row.try_get(0).unwrap_or(false);
        exists
    } else {
        false
    }
}

/////////////////////////////////////////////////////////
// Utility functions for inserting information into the database

/// Function to insert a block into the database
/// Database schema:
/// CREATE TABLE blocks (
/// "number" BIGINT NOT NULL PRIMARY KEY,
/// "hash" VARCHAR(66) NOT NULL,
/// "parentHash" VARCHAR(66) NOT NULL,
/// "nonce" VARCHAR(18) NOT NULL,
/// "sha3Uncles" VARCHAR(66) NOT NULL,
/// "logsBloom" TEXT NOT NULL,
/// "transactionsRoot" VARCHAR(66) NOT NULL,
/// "stateRoot" VARCHAR(66) NOT NULL,
/// "miner" VARCHAR(42) NOT NULL,
/// "difficulty" BIGINT NOT NULL,
/// "totalDifficulty" NUMERIC(50),
/// "size" INT NOT NULL,
/// "extraData" VARCHAR(66) NOT NULL,
/// "gasLimit" NUMERIC(100),
/// "gasUsed" NUMERIC(100),
/// "timestamp" INT NOT NULL,
/// "transactionsCount" INT,
/// "transactions_ids" JSON,
/// "uncles" JSON,
/// "lastUpdated" timestamp default current_timestamp
/// );
///
pub async fn insert_block(
    block: Block<H256>,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // println!(
    //     "Inserting block {} into database",
    //     block.number.unwrap().to_string()
    // );

    // Extract relevant data from the block
    let number = block.number.unwrap().as_u64() as i64;
    let hash = format!("0x{:x}", block.hash.unwrap());
    let parent_hash = format!("0x{:x}", block.parent_hash);
    let nonce = format!("0x{:x}", block.nonce.unwrap());
    let sha3_uncles = serde_json::to_value(&block.uncles).unwrap().to_string();
    let logs_bloom = format!("0x{:x}", block.logs_bloom.unwrap());
    let transactions_root = format!("0x{:x}", block.transactions_root);
    let state_root = format!("0x{:x}", block.state_root);
    let miner = format!("0x{:x}", block.author.unwrap());
    let difficulty = block.difficulty.as_u64() as i64;
    // let total_difficulty = block.total_difficulty.map(|d| Decimal::from(d.as_u64() as i64)).unwrap_or_default();
    let total_difficulty = block
        .total_difficulty
        .map(|u256| Decimal::from(u256.as_u128()))
        .unwrap_or(Decimal::new(0, 0));
    let size = block.size.unwrap().as_u32() as i32;
    let extra_data = format!("{:x}", block.extra_data);
    let gas_limit = Decimal::from(block.gas_limit.as_u128() as i64);
    let gas_used = Decimal::from(block.gas_used.as_u128() as i64);
    let timestamp = block.timestamp.as_u64() as i32;
    let transactions_count = block.transactions.len() as i32;
    let transactions_ids = serde_json::to_value(&block.transactions).unwrap();
    let uncles = serde_json::to_value(&block.uncles).unwrap();

    // Build the SQL query
    let query = r#"
        INSERT INTO blocks ("number", "hash", "parentHash", "nonce", "sha3Uncles", "logsBloom", "transactionsRoot",
                            "stateRoot", "miner", "difficulty", "totalDifficulty", "size", "extraData", "gasLimit",
                            "gasUsed", "timestamp", "transactionsCount", "transactions_ids", "uncles")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        ON CONFLICT ("number") DO NOTHING;
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
    let params: [&(dyn ToSql + Sync); 19] = [
        &number,
        &hash,
        &parent_hash,
        &nonce,
        &sha3_uncles,
        &logs_bloom,
        &transactions_root,
        &state_root,
        &miner,
        &difficulty,
        &total_difficulty,
        &size,
        &extra_data,
        &gas_limit,
        &gas_used,
        &timestamp,
        &transactions_count,
        &transactions_ids,
        &uncles,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Block {} inserted successfully", number);
            Ok(())
        }
        Err(err) => {
            eprintln!("Error inserting block {}: {}", number, err);
            Err(Box::new(err))
        }
    }
}

/// Function to insert a transaction into the database
/// Database schema:
/// CREATE TABLE transactions (
/// r VARCHAR(66) NOT NULL,
/// s VARCHAR(66) NOT NULL,
/// v VARCHAR(4) NOT NULL,
/// "to" VARCHAR(42),
/// "gas" INT NOT NULL,
/// "from" VARCHAR(42) NOT NULL,
/// "hash" VARCHAR(66) NOT NULL PRIMARY KEY,
/// "type" SMALLINT NOT NULL,
/// "input" TEXT NOT NULL,
/// "nonce" INT NOT NULL,
/// "value" NUMERIC(100),
/// "chainId" VARCHAR(10),
/// "gasPrice" NUMERIC(100),
/// "blockHash" VARCHAR(66),
/// "accessList" JSON,
/// "blockNumber" BIGINT NOT NULL,
/// "maxFeePerGas" NUMERIC(100),
/// "transactionIndex" INT NOT NULL,
/// "maxPriorityFeePerGas" NUMERIC(100),
/// "lastUpdated" timestamp default current_timestamp,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE
/// );
pub async fn insert_transaction(
    transaction: Transaction,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the transaction
    let r = format!("0x{:x}", transaction.r);
    let s = format!("0x{:x}", transaction.s);
    let v = format!("0x{:x}", transaction.v);
    let to = format!("0x{:x}", transaction.to.unwrap_or_default());
    let gas = transaction.gas.as_u64() as i32;
    let from = format!("0x{:x}", transaction.from);
    let hash = format!("0x{:x}", transaction.hash());
    let transaction_type = transaction.transaction_type.unwrap().as_u64() as i16;
    let input = format!("{:x}", transaction.input);
    let nonce = transaction.nonce.as_u64() as i32;
    let value = Decimal::from(transaction.value.as_u128() as i64);
    let chain_id = transaction.chain_id.unwrap().as_u64().to_string();
    let gas_price = Decimal::from(transaction.gas_price.unwrap().as_u128() as i64);
    let block_hash = format!("0x{:x}", transaction.block_hash.unwrap());
    let access_list = serde_json::to_value(&transaction.access_list).unwrap();
    let block_number = transaction.block_number.unwrap().as_u64() as i64;
    let max_fee_per_gas =
        Decimal::from(transaction.max_fee_per_gas.unwrap_or_default().as_u128() as i64);
    let transaction_index = transaction.transaction_index.unwrap_or_default().as_u64() as i32;
    let max_priority_fee_per_gas = Decimal::from(
        transaction
            .max_priority_fee_per_gas
            .unwrap_or_default()
            .as_u128() as i64,
    );

    // Build the SQL query
    let query = r#"
        INSERT INTO transactions ("r", "s", "v", "to", "gas", "from", "hash", "type", "input",
                                  "nonce", "value", "chainId", "gasPrice", "blockHash",
                                  "accessList", "blockNumber", "maxFeePerGas", "transactionIndex",
                                  "maxPriorityFeePerGas")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14 ,$15, $16, $17, $18, $19)
        ON CONFLICT ("hash") DO NOTHING;
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
    let params: [&(dyn ToSql + Sync); 19] = [
        &r,
        &s,
        &v,
        &to,
        &gas,
        &from,
        &hash,
        &transaction_type,
        &input,
        &nonce,
        &value,
        &chain_id,
        &gas_price,
        &block_hash,
        &access_list,
        &block_number,
        &max_fee_per_gas,
        &transaction_index,
        &max_priority_fee_per_gas,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Transaction {} inserted successfully", hash);
            Ok(())
        }
        Err(err) => {
            eprintln!("Error inserting transaction {}: {}", hash, err);
            Err(Box::new(err))
        }
    }
}

/// Function to insert a transaction receipt into the database
/// Database schema:
/// CREATE TABLE transactions_receipts (
/// "transactionHash" VARCHAR(66) NOT NULL PRIMARY KEY,
/// "transactionIndex" INT NOT NULL,
/// "blockHash" VARCHAR(66) NOT NULL,
/// "from" VARCHAR(42) NOT NULL,
/// "to" VARCHAR(42),
/// "blockNumber" BIGINT NOT NULL,
/// "cumulativeGasUsed" INT,
/// "gasUsed" INT,
/// "contractAddress" VARCHAR(42),
/// "logs" JSON,
/// "logsBloom" TEXT,
/// "status" BOOLEAN,
/// "effectiveGasPrice" VARCHAR(78),
/// "type" VARCHAR(10),
/// "lastUpdated" timestamp default current_timestamp,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE,
/// FOREIGN KEY ("transactionHash") REFERENCES transactions("hash") ON DELETE CASCADE
/// );
pub async fn insert_transaction_receipt(
    transaction_receipt: TransactionReceipt,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the transaction
    let transaction_hash = format!("0x{:x}", transaction_receipt.transaction_hash);
    let transaction_index = transaction_receipt.transaction_index.as_u64() as i32;
    let block_hash = format!("0x{:x}", transaction_receipt.block_hash.unwrap());
    let from = format!("0x{:x}", transaction_receipt.from);
    let to = format!("0x{:x}", transaction_receipt.to.unwrap_or_default());
    let block_number = transaction_receipt.block_number.unwrap().as_u64() as i64;
    let cumulative_gas_used = Decimal::from(transaction_receipt.cumulative_gas_used.as_u128());
    let gas_used = Decimal::from(transaction_receipt.gas_used.unwrap_or_default().as_u128() as i64);
    let contract_address = format!(
        "0x{:x}",
        transaction_receipt.contract_address.unwrap_or_default()
    );
    let logs = serde_json::to_value(&transaction_receipt.logs).unwrap();
    let logs_bloom = format!("0x{:x}", transaction_receipt.logs_bloom);
    let status = if transaction_receipt.status.unwrap_or_default().as_u32() == 1 {
        true
    } else {
        false
    };
    let effective_gas_price = Decimal::from(
        transaction_receipt
            .effective_gas_price
            .unwrap_or_default()
            .as_u128(),
    );
    let transaction_type = format!("{:?}", transaction_receipt.transaction_type.unwrap());

    // Build the SQL query
    let query = r#"
        INSERT INTO transactions_receipts ("transactionHash", "transactionIndex", "blockHash", "from",
                                            "to", "blockNumber", "cumulativeGasUsed", "gasUsed",
                                            "contractAddress", "logs", "logsBloom", "status",
                                            "effectiveGasPrice", "type")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13 ,$14)
        ON CONFLICT ("transactionHash") DO NOTHING;
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
    let params: [&(dyn ToSql + Sync); 14] = [
        &transaction_hash,
        &transaction_index,
        &block_hash,
        &from,
        &to,
        &block_number,
        &cumulative_gas_used,
        &gas_used,
        &contract_address,
        &logs,
        &logs_bloom,
        &status,
        &effective_gas_price,
        &transaction_type,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Transaction receipt {} inserted successfully", transaction_hash);
            Ok(())
        }
        Err(err) => {
            eprintln!(
                "Error inserting transaction receipt {}: {}",
                transaction_hash, err
            );
            Err(Box::new(err))
        }
    }
}

/// Function to insert an address into the database
/// Update the address if it already exists with this rules:
/// - If the block number is the same, do nothing
/// - If the block number is higher, update all fields
/// - If the block number is lower, do nothing
/// - If the block number is missing, do nothing
/// Database schema:
/// CREATE TABLE addresses (
/// "address" VARCHAR(42) NOT NULL PRIMARY KEY,
/// "balance" NUMERIC(100),
/// "nonce" INT,
/// "transactionCount" INT,
/// "blockNumber" BIGINT NOT NULL,
/// "contractCode" TEXT,
/// "gasUsed" INT,
/// "storage" VARCHAR(66),
/// "tokens" JSON,
/// "lastUpdated" timestamp default current_timestamp,
/// FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE
/// );
pub async fn insert_address(
    address: Address,
    balance: U256,
    nonce: U256,
    transaction_count: U256,
    storage: H256,
    code: Bytes,
    block_number: U64,
    _gas_used: U256, //TODO: handle the gas usage of an address
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    // Extract relevant data from the address
    let address = format!("0x{:x}", address);
    let balance = Decimal::from_parts(
        balance.low_u32() as u32, // lo
        0,                        // mid
        0,                        // hi
        false,                    // negative
        0,                        // scale
    );
    let nonce = nonce.as_u64() as i32;
    let transaction_count = transaction_count.as_u64() as i32;
    let storage = format!("0x{:x}", storage);
    let block_number = block_number.as_u64() as i64;
    let code = format!("{:x}", code);

    // Build the SQL query
    let query = r#"
        INSERT INTO addresses ("address", "balance", "nonce", "transactionCount", "blockNumber",
                               "contractCode", "storage")
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT ("address") DO UPDATE
        SET "balance" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."balance"
            ELSE addresses."balance"
            END,
            "nonce" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."nonce"
            ELSE addresses."nonce"
            END,
            "transactionCount" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."transactionCount"
            ELSE addresses."transactionCount"
            END,
            "blockNumber" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."blockNumber"
            ELSE addresses."blockNumber"
            END,
            "contractCode" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."contractCode"
            ELSE addresses."contractCode"
            END,
            "storage" = CASE
            WHEN excluded."blockNumber" > addresses."blockNumber" THEN excluded."storage"
            ELSE addresses."storage"
            END
        WHERE excluded."blockNumber" IS NOT NULL
        AND excluded."blockNumber" > addresses."blockNumber";
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
    let params: [&(dyn ToSql + Sync); 7] = [
        &address,
        &balance,
        &nonce,
        &transaction_count,
        &block_number,
        &code,
        &storage,
    ];

    // Execute the query with parameters
    let result = db_client.execute(&statement, &params).await;

    match result {
        Ok(_) => {
            // println!("Address {} inserted/updated successfully", address);
            Ok(())
        }
        Err(err) => {
            eprintln!("Error inserting/updating address {}: {}", address, err);
            Err(Box::new(err))
        }
    }
}

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
        verified_source_code["isProxy"].as_bool().unwrap_or_default()
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
        verified_source_code["optimizationUsed"].as_bool().unwrap_or_default()
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
