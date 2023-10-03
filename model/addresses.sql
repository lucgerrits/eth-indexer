CREATE TABLE addresses (
    "address" VARCHAR(42) NOT NULL PRIMARY KEY,
    "balance" NUMERIC(100),
    "nonce" INT,
    "transactionCount" INT,
    "blockNumber" BIGINT NOT NULL,
    "contractCode" TEXT,
    "gasUsed" INT,
    "storage" VARCHAR(66),
    "tokens" JSON,
    "lastUpdated" timestamp default current_timestamp,
    "insertedAt" timestamp,
    FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE
);
CREATE INDEX addresses_address_idx ON addresses ("address");