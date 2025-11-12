update blocks set consensus = false where number = 3;
INSERT INTO blocks (consensus, difficulty, gas_limit, gas_used, hash, miner_hash, nonce, number, parent_hash, size, "timestamp", total_difficulty, inserted_at, updated_at, refetch_needed, base_fee_per_gas, is_empty) VALUES (true, 0, 11533720, 1024160, '\x76a84109dc161bb4bbadbe88ee63c3622a584eadf630cc94ab7e051ce937e261', '\xbfcac5bf8d94d1517bc66604c6d09a7e9e050f5f', '\x0000000000000000', 3, '\x5cf02b50cc28fe3d3cdf8efcf4e39e411389cfe9469db5e738303cb3ecd06702', 928, '2025-07-24 15:28:22', NULL, '2025-07-24 15:28:28.512343', '2025-07-24 15:28:28.512343', false, 704298601, NULL);

update transactions set
  cumulative_gas_used = null,
  error = 'dropped/replaced',
  gas_used = null,
  index = null,
  status = 0,
  block_hash = null,
  block_number = null,
  max_priority_fee_per_gas = null,
  max_fee_per_gas = null,
  type = null,
  block_consensus = 'f'
where hash = '\x1872c3e9c4c76b5802b9a7c3f7798fac5bb8110d2707e145701acf90dd6de559';
INSERT INTO public.transactions (cumulative_gas_used, error, gas, gas_price, gas_used, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, created_contract_address_hash, created_contract_code_indexed_at, earliest_processing_start, old_block_hash, revert_reason, max_priority_fee_per_gas, max_fee_per_gas, type, has_error_in_internal_transactions, block_timestamp, block_consensus) VALUES (1024160, NULL, 1000000, 1704298601, 24160, '\x1932fed6f6464781ee6e928cf6b43a49d0dbb1024c9ac6c91ef480852c794cb9', 1,'\x0b2d80f859c0f853f851a0fa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d8a746578742f706c61696e6498746869732077696c6c2062652061667465722072656f7267c9c883666f6f83626172c0c0c0c003', 1, 101077921629596306466908745928446548255879322662257590841749393872699438920124, 2346445425930540296982349899175980964186148952107077648288543547740656147225, 1, 0, 0, '2025-07-24 15:28:28.512343', '2025-07-24 15:28:31.366921', '\x76a84109dc161bb4bbadbe88ee63c3622a584eadf630cc94ab7e051ce937e261', 3, '\xd29bb1a1a0f6d2783306a8618b3a5b58cb313152', '\x00000000000000000000000000000061726B6976', NULL, NULL, NULL, NULL, NULL, 1000000000, 5000000000, 2, false, '2025-07-24 15:28:22', true);

INSERT INTO public.logs (data, index, first_topic, second_topic, third_topic, fourth_topic, inserted_at, updated_at, address_hash, transaction_hash, block_hash, block_number) VALUES ('\x0000000000000000000000000000000000000000000000000000000000000067', 0, '\xf371f40aa6932ad9dacbee236e5f3b93d478afe3934b5cfec5ea0d800a41d165', '\xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d', NULL, NULL, '2025-07-24 15:28:28.512343', '2025-07-24 15:28:28.512343', '\x00000000000000000000000000000061726B6976', '\x1932fed6f6464781ee6e928cf6b43a49d0dbb1024c9ac6c91ef480852c794cb9', '\x76a84109dc161bb4bbadbe88ee63c3622a584eadf630cc94ab7e051ce937e261', 3);
