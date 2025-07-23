CREATE TABLE blocks (
    consensus boolean NOT NULL,
    difficulty numeric(50,0),
    gas_limit numeric(100,0) NOT NULL,
    gas_used numeric(100,0) NOT NULL,
    hash bytea NOT NULL,
    miner_hash bytea NOT NULL,
    nonce bytea NOT NULL,
    number bigint NOT NULL,
    parent_hash bytea NOT NULL,
    size integer,
    "timestamp" timestamp without time zone NOT NULL,
    total_difficulty numeric(50,0),
    inserted_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    refetch_needed boolean DEFAULT false,
    base_fee_per_gas numeric(100,0),
    is_empty boolean
);

CREATE TABLE logs (
    data bytea NOT NULL,
    index integer NOT NULL,
    first_topic bytea,
    second_topic bytea,
    third_topic bytea,
    fourth_topic bytea,
    inserted_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    address_hash bytea,
    transaction_hash bytea NOT NULL,
    block_hash bytea NOT NULL,
    block_number integer
);

CREATE TABLE transactions (
    cumulative_gas_used numeric(100,0),
    error character varying(255),
    gas numeric(100,0) NOT NULL,
    gas_price numeric(100,0),
    gas_used numeric(100,0),
    hash bytea NOT NULL,
    index integer,
    input bytea NOT NULL,
    nonce integer NOT NULL,
    r numeric(100,0) NOT NULL,
    s numeric(100,0) NOT NULL,
    status integer,
    v numeric(100,0) NOT NULL,
    value numeric(100,0) NOT NULL,
    inserted_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    block_hash bytea,
    block_number integer,
    from_address_hash bytea NOT NULL,
    to_address_hash bytea,
    created_contract_address_hash bytea,
    created_contract_code_indexed_at timestamp without time zone,
    earliest_processing_start timestamp without time zone,
    old_block_hash bytea,
    revert_reason text,
    max_priority_fee_per_gas numeric(100,0),
    max_fee_per_gas numeric(100,0),
    type integer,
    has_error_in_internal_transactions boolean,
    block_timestamp timestamp without time zone,
    block_consensus boolean DEFAULT true,
    CONSTRAINT collated_block_number CHECK (((block_hash IS NULL) OR (block_number IS NOT NULL))),
    CONSTRAINT collated_cumalative_gas_used CHECK (((block_hash IS NULL) OR (cumulative_gas_used IS NOT NULL))),
    CONSTRAINT collated_gas_price CHECK (((block_hash IS NULL) OR (gas_price IS NOT NULL))),
    CONSTRAINT collated_gas_used CHECK (((block_hash IS NULL) OR (gas_used IS NOT NULL))),
    CONSTRAINT collated_index CHECK (((block_hash IS NULL) OR (index IS NOT NULL))),
    CONSTRAINT error CHECK (((status = 0) OR ((status <> 0) AND (error IS NULL)))),
    CONSTRAINT pending_block_number CHECK (((block_hash IS NOT NULL) OR (block_number IS NULL))),
    CONSTRAINT pending_cumalative_gas_used CHECK (((block_hash IS NOT NULL) OR (cumulative_gas_used IS NULL))),
    CONSTRAINT pending_gas_used CHECK (((block_hash IS NOT NULL) OR (gas_used IS NULL))),
    CONSTRAINT pending_index CHECK (((block_hash IS NOT NULL) OR (index IS NULL))),
    CONSTRAINT status CHECK ((((block_hash IS NULL) AND (status IS NULL)) OR (block_hash IS NOT NULL) OR ((status = 0) AND ((error)::text = 'dropped/replaced'::text))))
);

ALTER TABLE ONLY public.blocks
    ADD CONSTRAINT blocks_pkey PRIMARY KEY (hash);

ALTER TABLE ONLY logs
    ADD CONSTRAINT logs_pkey PRIMARY KEY (transaction_hash, block_hash, index);

ALTER TABLE ONLY transactions
    ADD CONSTRAINT transactions_pkey PRIMARY KEY (hash);

CREATE INDEX blocks_consensus_index ON public.blocks USING btree (consensus);
CREATE INDEX blocks_date ON public.blocks USING btree (date("timestamp"), number);
CREATE INDEX blocks_inserted_at_index ON public.blocks USING btree (inserted_at);
CREATE INDEX blocks_is_empty_index ON public.blocks USING btree (is_empty);
CREATE INDEX blocks_miner_hash_index ON public.blocks USING btree (miner_hash);
CREATE INDEX blocks_miner_hash_number_index ON public.blocks USING btree (miner_hash, number);
CREATE INDEX blocks_number_index ON public.blocks USING btree (number);
CREATE INDEX blocks_timestamp_index ON public.blocks USING btree ("timestamp");
CREATE INDEX consensus_block_hashes_refetch_needed ON public.blocks USING btree (hash) WHERE (consensus AND refetch_needed);
CREATE INDEX empty_consensus_blocks ON public.blocks USING btree (consensus) WHERE (is_empty IS NULL);
CREATE UNIQUE INDEX one_consensus_block_at_height ON public.blocks USING btree (number) WHERE consensus;
CREATE UNIQUE INDEX one_consensus_child_per_parent ON public.blocks USING btree (parent_hash) WHERE consensus;
CREATE INDEX "logs_address_hash_block_number_DESC_index_DESC_index" ON logs USING btree (address_hash, block_number DESC, index DESC);
CREATE INDEX logs_block_hash_index ON logs USING btree (block_hash);
CREATE INDEX "logs_block_number_DESC__index_DESC_index" ON logs USING btree (block_number DESC, index DESC);
CREATE INDEX logs_deposits_withdrawals_index ON logs USING btree (transaction_hash, block_hash, index, address_hash) WHERE (first_topic = ANY (ARRAY['\xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c'::bytea, '\x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65'::bytea]));
CREATE INDEX logs_first_topic_index ON logs USING btree (first_topic);
CREATE INDEX logs_fourth_topic_index ON logs USING btree (fourth_topic);
CREATE INDEX logs_second_topic_index ON logs USING btree (second_topic);
CREATE INDEX logs_third_topic_index ON logs USING btree (third_topic);
CREATE INDEX logs_transaction_hash_index_index ON logs USING btree (transaction_hash, index);
CREATE INDEX method_id ON transactions USING btree (SUBSTRING(input FROM 1 FOR 4));
CREATE INDEX pending_txs_index ON transactions USING btree (inserted_at, hash) WHERE ((block_hash IS NULL) AND ((error IS NULL) OR ((error)::text <> 'dropped/replaced'::text)));
CREATE INDEX transactions_block_consensus_index ON transactions USING btree (block_consensus);
CREATE INDEX transactions_block_hash_error_index ON transactions USING btree (block_hash, error);
CREATE UNIQUE INDEX transactions_block_hash_index_index ON transactions USING btree (block_hash, index);
CREATE INDEX transactions_block_number_index ON transactions USING btree (block_number);
CREATE INDEX transactions_block_timestamp_index ON transactions USING btree (block_timestamp);
CREATE INDEX transactions_created_contract_address_hash_with_pending_index_a ON transactions USING btree (created_contract_address_hash, block_number, index, inserted_at, hash DESC);
CREATE INDEX transactions_created_contract_code_indexed_at_index ON transactions USING btree (created_contract_code_indexed_at);
CREATE INDEX transactions_from_address_hash_with_pending_index_asc ON transactions USING btree (from_address_hash, block_number, index, inserted_at, hash DESC);
CREATE INDEX transactions_inserted_at_index ON transactions USING btree (inserted_at);
CREATE INDEX transactions_nonce_from_address_hash_block_hash_index ON transactions USING btree (nonce, from_address_hash, block_hash);
CREATE INDEX transactions_recent_collated_index ON transactions USING btree (block_number DESC, index DESC);
CREATE INDEX transactions_status_index ON transactions USING btree (status);
CREATE INDEX transactions_to_address_hash_with_pending_index_asc ON transactions USING btree (to_address_hash, block_number, index, inserted_at, hash DESC);
CREATE INDEX transactions_updated_at_index ON transactions USING btree (updated_at);
ALTER TABLE ONLY logs
    ADD CONSTRAINT logs_transaction_hash_fkey FOREIGN KEY (transaction_hash) REFERENCES transactions(hash) ON DELETE CASCADE;

