#Run from project root path:
# docker build -f Dockerfile -t local/eth-indexer .
FROM ubuntu:22.04 as builder

RUN apt-get update \
    && apt-get install -y \
    curl \
    build-essential \
    llvm \
    clang \
    libudev-dev \
    libssl-dev \
    cmake \
    dnsutils \
    pkg-config \
    protobuf-compiler

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y 

ENV PATH=$PATH:/root/.cargo/bin

RUN rustup default stable
RUN rustup update
RUN rustup update nightly
RUN rustup override set nightly-2023-04-05

COPY . /eth-indexer

WORKDIR /eth-indexer

RUN cargo build --release

FROM ubuntu:22.04 AS runner

RUN mkdir /eth-indexer

WORKDIR /eth-indexer

COPY --from=builder /eth-indexer/target/release/eth-indexer ./eth-indexer
COPY --from=builder /eth-indexer/.env.production ./.env.production

# 9615 for Prometheus (metrics)
# TODO: Add prometheus metrics support
EXPOSE 9615 

ENV ETH_INDEXER=production

RUN ./eth-indexer --help

CMD [ "./eth-indexer index_live" ]
