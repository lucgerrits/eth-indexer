CREATE TABLE contracts (
    "address" VARCHAR(42) NOT NULL PRIMARY KEY,
    "blockNumber" BIGINT NOT NULL,
    "transactionHash" VARCHAR(66) NOT NULL,
    "creatorAddress" VARCHAR(42) NOT NULL,
    "contractType" character varying(255),
    "abi" JSON,
    "sourceCode" TEXT,
    "additionalSources" TEXT,
    "compilerSettings" TEXT,
    "constructorArguments" TEXT,
    "EVMVersion" TEXT,
    "fileName" TEXT,
    "isProxy" BOOLEAN,
    "contractName" TEXT,
    "compilerVersion" TEXT,
    "optimizationUsed" BOOLEAN,
    "bytecode" TEXT,
    FOREIGN KEY ("blockNumber") REFERENCES blocks("number") ON DELETE CASCADE,
    FOREIGN KEY ("transactionHash") REFERENCES transactions("hash") ON DELETE CASCADE
);

CREATE INDEX contracts_address_idx ON contracts ("address");