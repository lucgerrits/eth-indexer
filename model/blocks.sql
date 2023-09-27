CREATE TABLE blocks (
    "number" BIGINT NOT NULL PRIMARY KEY,
    "hash" VARCHAR(66) NOT NULL,
    "parentHash" VARCHAR(66) NOT NULL,
    "nonce" VARCHAR(18) NOT NULL,
    "sha3Uncles" VARCHAR(66) NOT NULL,
    "logsBloom" TEXT NOT NULL,
    "transactionsRoot" VARCHAR(66) NOT NULL,
    "stateRoot" VARCHAR(66) NOT NULL,
    "miner" VARCHAR(42) NOT NULL,
    "difficulty" BIGINT NOT NULL,
    "totalDifficulty" NUMERIC(50),
    "size" INT NOT NULL,
    "extraData" VARCHAR(66) NOT NULL,
    "gasLimit" NUMERIC(100),
    "gasUsed" NUMERIC(100),
    "timestamp" INT NOT NULL,
    "transactionsCount" INT,
    "transactions_ids" JSON,
    "uncles" JSON,
    "lastUpdated" timestamp default current_timestamp
);

CREATE INDEX blocks_number_idx ON blocks ("number");