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

ALTER TABLE ONLY public.transactions
    ADD CONSTRAINT transactions_pkey PRIMARY KEY (hash);
CREATE INDEX method_id ON public.transactions USING btree (SUBSTRING(input FROM 1 FOR 4));
CREATE INDEX pending_txs_index ON public.transactions USING btree (inserted_at, hash) WHERE ((block_hash IS NULL) AND ((error IS NULL) OR ((error)::text <> 'dropped/replaced'::text)));
CREATE INDEX transactions_block_consensus_index ON public.transactions USING btree (block_consensus);
CREATE INDEX transactions_block_hash_error_index ON public.transactions USING btree (block_hash, error);
CREATE UNIQUE INDEX transactions_block_hash_index_index ON public.transactions USING btree (block_hash, index);
CREATE INDEX transactions_block_number_index ON public.transactions USING btree (block_number);
CREATE INDEX transactions_block_timestamp_index ON public.transactions USING btree (block_timestamp);
CREATE INDEX transactions_created_contract_address_hash_with_pending_index_a ON public.transactions USING btree (created_contract_address_hash, block_number, index, inserted_at, hash DESC);
CREATE INDEX transactions_created_contract_code_indexed_at_index ON public.transactions USING btree (created_contract_code_indexed_at);
CREATE INDEX transactions_from_address_hash_with_pending_index_asc ON public.transactions USING btree (from_address_hash, block_number, index, inserted_at, hash DESC);
CREATE INDEX transactions_inserted_at_index ON public.transactions USING btree (inserted_at);
CREATE INDEX transactions_nonce_from_address_hash_block_hash_index ON public.transactions USING btree (nonce, from_address_hash, block_hash);
CREATE INDEX transactions_recent_collated_index ON public.transactions USING btree (block_number DESC, index DESC);
CREATE INDEX transactions_status_index ON public.transactions USING btree (status);
CREATE INDEX transactions_to_address_hash_with_pending_index_asc ON public.transactions USING btree (to_address_hash, block_number, index, inserted_at, hash DESC);
CREATE INDEX transactions_updated_at_index ON public.transactions USING btree (updated_at);
