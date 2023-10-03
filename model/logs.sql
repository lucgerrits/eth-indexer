CREATE TABLE logs (
    "data"Bytea,
    "index" integer,
    "type" VARCHAR(255),
    "firstTopic" VARCHAR(255),
    "secondTopic" VARCHAR(255),
    "thirdTopic" VARCHAR(255),
    "fourthTopic" VARCHAR(255),
    "address" VARCHAR(42) NOT NULL,
    "transactionHash" VARCHAR(66) NOT NULL,
    "blockHash" VARCHAR(66) NOT NULL,
    "blockNumber" BIGINT NOT NULL,
    "insertedAt" timestamp,
    "updatedAt" timestamp default current_timestamp,
    CONSTRAINT logs_pkey PRIMARY KEY ("transactionHash", "blockHash", "index")
);
CREATE INDEX logs_address_hash_transaction_hash_index ON logs ("address", "transactionHash", "blockHash");