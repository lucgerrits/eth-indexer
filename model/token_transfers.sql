CREATE TABLE "token_transfers" (
    "contractAddress" VARCHAR(42) NOT NULL,
    "fromAddress" VARCHAR(42),
    "toAddress" VARCHAR(42),
    "transactionHash" VARCHAR(66) NOT NULL,
    "blockNumber" BIGINT NOT NULL,
    "blockHash" VARCHAR(66),
    "logIndex" integer NOT NULL,
    "amount" NUMERIC(100),
    "insertedAt" timestamp,
    "lastUpdated" timestamp default current_timestamp,
    CONSTRAINT token_transfers_pkey PRIMARY KEY ("transactionHash", "blockHash", "logIndex")
);