CREATE TABLE tokens (
    "address" VARCHAR(42) NOT NULL PRIMARY KEY,
    "type" character varying(255) NOT NULL,
    "name" text,
    "symbol" text,
    "totalSupply" numeric,
    "decimals" numeric,
    "lastUpdated" timestamp default current_timestamp,
    "holderCount" integer,
    "totalSupplyUpdatedAtBlock" bigint
);
CREATE INDEX tokens_address_idx ON tokens ("address");
