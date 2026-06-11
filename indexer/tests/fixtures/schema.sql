-- Test schema synced with the entity definitions in
-- src/infrastructure/persistence/entities/
-- Mirrors the columns of the production schema applied by ../../database/
-- so tests run against an ephemeral Postgres without external dependencies.

CREATE TABLE charms (
    txid                TEXT        NOT NULL,
    vout                INTEGER     NOT NULL,
    block_height        INTEGER,
    data                JSONB       NOT NULL DEFAULT '{}'::jsonb,
    date_created        TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    asset_type          TEXT        NOT NULL,
    blockchain          TEXT        NOT NULL,
    network             TEXT        NOT NULL,
    address             TEXT,
    spent               BOOLEAN     NOT NULL DEFAULT FALSE,
    app_id              TEXT        NOT NULL,
    amount              BIGINT      NOT NULL DEFAULT 0,
    mempool_detected_at TIMESTAMPTZ,
    tags                TEXT,
    verified            BOOLEAN     NOT NULL DEFAULT TRUE,
    PRIMARY KEY (txid, vout)
);

CREATE TABLE transactions (
    txid                TEXT        NOT NULL PRIMARY KEY,
    block_height        INTEGER,
    ordinal             BIGINT      NOT NULL,
    raw                 JSONB       NOT NULL DEFAULT '{}'::jsonb,
    charm               JSONB       NOT NULL DEFAULT '{}'::jsonb,
    updated_at          TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status              TEXT        NOT NULL DEFAULT 'pending',
    confirmations       INTEGER     NOT NULL DEFAULT 0,
    blockchain          TEXT        NOT NULL,
    network             TEXT        NOT NULL,
    mempool_detected_at TIMESTAMPTZ,
    tags                TEXT,
    tx_type             TEXT
);

CREATE TABLE assets (
    id                       SERIAL PRIMARY KEY,
    app_id                   TEXT        NOT NULL UNIQUE,
    txid                     TEXT        NOT NULL,
    vout_index               INTEGER     NOT NULL,
    charm_id                 TEXT        NOT NULL,
    block_height             INTEGER     NOT NULL,
    date_created             TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    data                     JSONB       NOT NULL DEFAULT '{}'::jsonb,
    asset_type               TEXT        NOT NULL,
    blockchain               TEXT        NOT NULL,
    network                  TEXT        NOT NULL,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name                     TEXT,
    symbol                   TEXT,
    description              TEXT,
    image_url                TEXT,
    total_supply             NUMERIC(30, 0),
    decimals                 SMALLINT    NOT NULL DEFAULT 8,
    is_reference_nft         BOOLEAN     NOT NULL DEFAULT FALSE,
    cardano_policy_id        TEXT,
    cardano_asset_name       TEXT,
    cardano_fingerprint      TEXT
);

CREATE TABLE summary (
    id                            SERIAL PRIMARY KEY,
    network                       TEXT        NOT NULL UNIQUE,
    last_processed_block          INTEGER     NOT NULL DEFAULT 0,
    latest_confirmed_block        INTEGER     NOT NULL DEFAULT 0,
    total_charms                  BIGINT      NOT NULL DEFAULT 0,
    total_transactions            BIGINT      NOT NULL DEFAULT 0,
    confirmed_transactions        BIGINT      NOT NULL DEFAULT 0,
    confirmation_rate             INTEGER     NOT NULL DEFAULT 0,
    nft_count                     BIGINT      NOT NULL DEFAULT 0,
    token_count                   BIGINT      NOT NULL DEFAULT 0,
    dapp_count                    BIGINT      NOT NULL DEFAULT 0,
    other_count                   BIGINT      NOT NULL DEFAULT 0,
    bitcoin_node_status           TEXT        NOT NULL DEFAULT 'unknown',
    bitcoin_node_block_count      BIGINT      NOT NULL DEFAULT 0,
    bitcoin_node_best_block_hash  TEXT        NOT NULL DEFAULT 'unknown',
    last_updated                  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at                    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    charms_cast_count             BIGINT      NOT NULL DEFAULT 0,
    bro_count                     BIGINT      NOT NULL DEFAULT 0,
    dex_orders_count              BIGINT      NOT NULL DEFAULT 0
);

CREATE TABLE stats_holders (
    id                  SERIAL PRIMARY KEY,
    app_id              TEXT        NOT NULL,
    address             TEXT        NOT NULL,
    network             TEXT        NOT NULL DEFAULT 'mainnet',
    total_amount        BIGINT      NOT NULL DEFAULT 0,
    charm_count         INTEGER     NOT NULL DEFAULT 0,
    first_seen_block    INTEGER     NOT NULL DEFAULT 0,
    last_updated_block  INTEGER     NOT NULL DEFAULT 0,
    created_at          TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (app_id, address, network)
);

CREATE TABLE address_utxos (
    txid          TEXT    NOT NULL,
    vout          INTEGER NOT NULL,
    network       TEXT    NOT NULL,
    address       TEXT    NOT NULL,
    value         BIGINT  NOT NULL,
    script_pubkey TEXT    NOT NULL DEFAULT '',
    block_height  INTEGER,
    PRIMARY KEY (txid, vout, network)
);

CREATE TABLE address_transactions (
    txid          VARCHAR(64)  NOT NULL,
    address       VARCHAR(128) NOT NULL,
    network       VARCHAR(16)  NOT NULL,
    direction     VARCHAR(8)   NOT NULL,
    amount        BIGINT       NOT NULL,
    fee           BIGINT       NOT NULL DEFAULT 0,
    block_height  INTEGER,
    block_time    BIGINT,
    confirmations INTEGER      NOT NULL DEFAULT 0,
    PRIMARY KEY (txid, address, network)
);

CREATE TABLE block_status (
    block_height   INTEGER     NOT NULL,
    network        TEXT        NOT NULL,
    blockchain     TEXT        NOT NULL,
    downloaded     BOOLEAN     NOT NULL DEFAULT FALSE,
    processed      BOOLEAN     NOT NULL DEFAULT FALSE,
    confirmed      BOOLEAN     NOT NULL DEFAULT FALSE,
    block_hash     TEXT,
    tx_count       INTEGER,
    charm_count    INTEGER,
    downloaded_at  TIMESTAMPTZ,
    processed_at   TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (block_height, network, blockchain)
);

CREATE TABLE dex_orders (
    order_id          TEXT      NOT NULL PRIMARY KEY,
    txid              TEXT      NOT NULL,
    vout              INTEGER   NOT NULL,
    block_height      INTEGER,
    platform          TEXT      NOT NULL,
    maker             TEXT      NOT NULL,
    side              TEXT      NOT NULL,
    exec_type         TEXT      NOT NULL,
    price_num         BIGINT    NOT NULL,
    price_den         BIGINT    NOT NULL,
    amount            BIGINT    NOT NULL,
    quantity          BIGINT    NOT NULL,
    filled_amount     BIGINT    NOT NULL DEFAULT 0,
    filled_quantity   BIGINT    NOT NULL DEFAULT 0,
    asset_app_id      TEXT      NOT NULL,
    scrolls_address   TEXT,
    status            TEXT      NOT NULL,
    parent_order_id   TEXT,
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    blockchain        TEXT      NOT NULL,
    network           TEXT      NOT NULL
);

CREATE TABLE mempool_spends (
    spent_txid    TEXT        NOT NULL,
    spent_vout    INTEGER     NOT NULL,
    network       TEXT        NOT NULL,
    spending_txid TEXT        NOT NULL,
    detected_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (spent_txid, spent_vout, network)
);

CREATE TABLE monitored_addresses (
    address      TEXT        NOT NULL,
    network      TEXT        NOT NULL,
    source       TEXT        NOT NULL,
    seeded_at    TIMESTAMPTZ,
    seed_height  INTEGER,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (address, network)
);
