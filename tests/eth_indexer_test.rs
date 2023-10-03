// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tokio::sync::oneshot;
//     use std::thread;

//     #[tokio::test]
//     async fn test_indexer() {
//         // Set up the environment
//         dotenv().ok();
//         let http_rpc_endpoint = env::var("HTTP_RPC_ENDPOINT").unwrap();
//         let db_url = env::var("DATABASE_URL").unwrap();

//         // Start the indexer in a separate thread
//         let (tx, rx) = oneshot::channel();
//         let indexer_thread = thread::spawn(move || {
//             let indexer = Indexer::new(http_rpc_endpoint, db_url);
//             let result = indexer.run();
//             tx.send(result).unwrap();
//         });

//         // Wait for the indexer to finish
//         let result = rx.await.unwrap();
//         assert!(result.is_ok());

//         // Join the indexer thread
//         indexer_thread.join().unwrap();
//     }
// }