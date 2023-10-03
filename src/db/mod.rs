// Module to handle postgress database
// db/mod.rs
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use log::{error as log_error, info};
use std::env;
use std::error::Error;
use std::fs;
use tokio_postgres::{Client as PostgresClient, NoTls};

mod blocks;
pub use blocks::*;

mod transactions;
pub use transactions::*;

mod addresses;
pub use addresses::*;

mod contracts;
pub use contracts::*;

mod tokens;
pub use tokens::*;

mod logs;
pub use logs::*;

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

    info!("Connected to database!");
    pool
}

async fn check_database_exists(url: &str, database_name: &str) -> bool {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("Failed to connect to the database for checking existence");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            log_error!("database connection error for existence check: {}", e);
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

/// Helper function to create a database
async fn create_database(
    host: &str,
    port: &str,
    user: &str,
    password: &str,
    database: &str,
    url: &str,
) -> Result<PostgresClient, tokio_postgres::Error> {
    info!(
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
            log_error!("Default database connection error: {}", e);
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
            log_error!("New database connection error: {}", e);
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
                // println!("Database is up-to-date. Skipping initialization.");
                return Ok(());
            }
        }
    }

    // If the table doesn't exist or the versions don't match, perform initialization
    // TODO: perform an update instead on just applying the SQL files
    let sql_files = env::var("POSTGRES_CREATE_TABLE_ORDER").unwrap();
    let sql_file_paths: Vec<&str> = sql_files.split(",").collect();
    for sql_file_path in sql_file_paths {
        let full_sql_file_path = format!("./model/{}.sql", sql_file_path);
        info!("executing sql file: {}", full_sql_file_path);
        let sql = match fs::read_to_string(&full_sql_file_path) {
            Ok(sql) => sql,
            Err(e) => panic!("Error reading sql file: {}", e),
        };

        // Execute SQL queries using prepared statements
        if let Err(e) = db_client.batch_execute(&sql).await {
            log_error!("Error executing SQL from {}: {}", sql_file_path, e);
        } else {
            info!("Executed SQL from: {:?}", sql_file_path);
        }
    }

    // Update the version configuration
    let update_version_query = format!(
        "INSERT INTO configuration (config_name, config_value) VALUES ('{}', '{}')
         ON CONFLICT (config_name) DO UPDATE SET config_value = EXCLUDED.config_value",
        config_name, config_version
    );

    if let Err(e) = db_client.batch_execute(&update_version_query).await {
        log_error!("Error updating version in configuration: {}", e);
    }

    Ok(())
}

/// Helper function to check if a table exists
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


async fn get_abi_by_address(
    address: String,
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let db_client = db_pool.get().await?;
    let query = format!(
        "SELECT \"abi\" FROM contracts WHERE address = '{}'",
        address
    );
    let row = db_client.query_one(&query, &[]).await?;
    let abi_json: serde_json::Value = row.try_get("abi")?;
    if abi_json.is_null() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "abi is null",
        )));
    }
    Ok(abi_json)
}