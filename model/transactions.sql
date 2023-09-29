CREATE TABLE transactions (
    r VARCHAR(66) NOT NULL,
    s VARCHAR(66) NOT NULL,
    v VARCHAR(4) NOT NULL,
    "to" VARCHAR(42),
    "gas" INT NOT NULL,
    "from" VARCHAR(42) NOT NULL,
    "hash" VARCHAR(66) NOT NULL PRIMARY KEY,
    "type" SMALLINT NOT NULL,
    "input" TEXT NOT NULL,
    "nonce" INT NOT NULL,
    "value" NUMERIC(100),
    "chainId" VARCHAR(10),
    "gasPrice" NUMERIC(100),
    "blockHash" VARCHAR(66),
    "accessList" JSON,
    "blockNumber" BIGINT NOT NULL,
    "maxFeePerGas" NUMERIC(100),
    "transactionIndex" INT NOT NULL,
    "maxPriorityFeePerGas" NUMERIC(100),
    "lastUpdated" timestamp default current_timestamp,
    FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE
);
CREATE INDEX transactions_hash_idx ON transactions ("hash");