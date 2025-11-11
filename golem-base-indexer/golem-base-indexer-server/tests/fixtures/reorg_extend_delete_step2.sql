update blocks set consensus = false where number = 3;
INSERT INTO blocks (consensus, difficulty, gas_limit, gas_used, hash, miner_hash, nonce, number, parent_hash, size, "timestamp", total_difficulty, inserted_at, updated_at, refetch_needed, base_fee_per_gas, is_empty) VALUES (true, 0, 11533720, 1022520, '\x8aa0fcf1219e4738b6d218070903dba819b3168475866fbe15610f9fdf09c94e', '\x98096c47aed77010ec80fe96190f3b87db5c92a9', '\x0000000000000000', 3, '\x189de1512acd5564e70357a0655d3e02d373653eb207e3cf9691181371b0569c', 885, '2025-07-24 16:16:00', NULL, '2025-07-24 16:16:06.51129', '2025-07-24 16:16:06.51129', false, 704298601, NULL);

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
where hash = '\x488a9a57364c22e819a6af41fca5db893a2dee1f678d859ec6bd5079aae71453';

INSERT INTO transactions (cumulative_gas_used, error, gas, gas_price, gas_used, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, created_contract_address_hash, created_contract_code_indexed_at, earliest_processing_start, old_block_hash, revert_reason, max_priority_fee_per_gas, max_fee_per_gas, type, has_error_in_internal_transactions, block_timestamp, block_consensus) VALUES (1022520, NULL, 1000000, 1704298601, 22520, '\xdac82fe3f61d518aefddb840e859699f50ab0713ce1ab0c0123ebbcee05fb325', 1,'\x0b1380e6c0c0e1a0fa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95dc0c003', 1, 10973917075442819811816046085998710655763210408601392249198537256596855012343, 52877116905673805403813908332153165425176200296999064200948463879795820432377, 1, 0, 0, '2025-07-24 16:16:06.51129', '2025-07-24 16:16:07.298554', '\x8aa0fcf1219e4738b6d218070903dba819b3168475866fbe15610f9fdf09c94e', 3, '\xd29bb1a1a0f6d2783306a8618b3a5b58cb313152', '\x00000000000000000000000000000061726B6976', NULL, NULL, NULL, NULL, NULL, 1000000000, 5000000000, 2, false, '2025-07-24 16:16:00', true);

INSERT INTO logs (data, index, first_topic, second_topic, third_topic, fourth_topic, inserted_at, updated_at, address_hash, transaction_hash, block_hash, block_number) VALUES ('\x', 0, '\x749d62eff980a5016f4f357bd7eb8b65163f1e25bc400dcfc5e33f0e7910149e', '\xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d', NULL, NULL, '2025-07-24 16:16:06.51129', '2025-07-24 16:16:06.51129', '\x00000000000000000000000000000061726B6976', '\xdac82fe3f61d518aefddb840e859699f50ab0713ce1ab0c0123ebbcee05fb325', '\x8aa0fcf1219e4738b6d218070903dba819b3168475866fbe15610f9fdf09c94e', 3);

