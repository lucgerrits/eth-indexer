CREATE TABLE transactions_receipts (
    "transactionHash" VARCHAR(66) NOT NULL PRIMARY KEY,
    "transactionIndex" INT NOT NULL,
    "blockHash" VARCHAR(66) NOT NULL,
    "from" VARCHAR(42) NOT NULL,
    "to" VARCHAR(42),
    "blockNumber" BIGINT NOT NULL,
    "cumulativeGasUsed" NUMERIC(100),
    "gasUsed" NUMERIC(100),
    "contractAddress" VARCHAR(42),
    "logs" JSON,
    "logsBloom" TEXT,
    "status" BOOLEAN,
    "effectiveGasPrice" NUMERIC(100),
    "type" VARCHAR(10),
    "insertedAt" timestamp,
    "lastUpdated" timestamp default current_timestamp,
    FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE,
    FOREIGN KEY ("transactionHash") REFERENCES transactions("hash") ON DELETE CASCADE
);
CREATE INDEX transactions_receipts_contractAddress_idx ON transactions_receipts ("contractAddress");