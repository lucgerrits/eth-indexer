CREATE TABLE tokens (
    "address" VARCHAR(42) NOT NULL PRIMARY KEY,
    "type" character varying(255) NOT NULL,
    "name" text,
    "symbol" text,
    "totalSupply" numeric,
    "decimals" numeric,
    "holderCount" integer,
    "totalSupplyUpdatedAtBlock" bigint,
    "insertedAt" timestamp,
    "lastUpdated" timestamp default current_timestamp
);
CREATE INDEX tokens_address_idx ON tokens ("address");
