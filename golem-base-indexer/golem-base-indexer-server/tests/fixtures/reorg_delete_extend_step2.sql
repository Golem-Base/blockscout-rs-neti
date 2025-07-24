update blocks set consensus = false where number = 3;
INSERT INTO public.blocks (consensus, difficulty, gas_limit, gas_used, hash, miner_hash, nonce, number, parent_hash, size, "timestamp", total_difficulty, inserted_at, updated_at, refetch_needed, base_fee_per_gas, is_empty) VALUES (true, 0, 11533720, 1022600, '\x2f2bbb7b91c28dab8b7b56323c29526ab45ff33bbb91009141c4642979fefa8b', '\x07c2079fe283bb4e318f2907ab1e21c02ac869db', '\x0000000000000000', 3, '\xe3e1010f0c2067b9e157d46f0cbd7fdd1d894023b8b9aedbc9814e78da6d3a90', 881, '2025-07-24 15:37:18', NULL, '2025-07-24 15:37:23.850646', '2025-07-24 15:37:23.850646', false, 704298601, NULL);


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
where hash = '\xdac82fe3f61d518aefddb840e859699f50ab0713ce1ab0c0123ebbcee05fb325';
INSERT INTO public.transactions (cumulative_gas_used, error, gas, gas_price, gas_used, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, created_contract_address_hash, created_contract_code_indexed_at, earliest_processing_start, old_block_hash, revert_reason, max_priority_fee_per_gas, max_fee_per_gas, type, has_error_in_internal_transactions, block_timestamp, block_consensus) VALUES (1022600, NULL, 22936, 704298602, 22600, '\x488a9a57364c22e819a6af41fca5db893a2dee1f678d859ec6bd5079aae71453', 1, '\xe7c0c0c0e3e2a0fa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d64', 1, 64162084688818061067310461725301729103729577919551830520755981447750826617429, 17181882848647383606901759477469333658873227460046785878272648537678196757531, 1, 1, 0, '2025-07-24 15:37:23.850646', '2025-07-24 15:37:24.030244', '\x2f2bbb7b91c28dab8b7b56323c29526ab45ff33bbb91009141c4642979fefa8b', 3, '\xd29bb1a1a0f6d2783306a8618b3a5b58cb313152', '\x0000000000000000000000000000000060138453', NULL, NULL, NULL, NULL, NULL, 1, 1570054507, 2, false, '2025-07-24 15:37:18', true);


INSERT INTO public.logs (data, index, first_topic, second_topic, third_topic, fourth_topic, inserted_at, updated_at, address_hash, transaction_hash, block_hash, block_number) VALUES ('\x000000000000000000000000000000000000000000000000000000000000007d00000000000000000000000000000000000000000000000000000000000000e1', 0, '\x835bfca6df78ffac92635dcc105a6a8c4fd715e054e18ef60448b0a6dce30c8d', '\xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d', NULL, NULL, '2025-07-24 15:37:23.850646', '2025-07-24 15:37:23.850646', '\x0000000000000000000000000000000060138453', '\x488a9a57364c22e819a6af41fca5db893a2dee1f678d859ec6bd5079aae71453', '\x2f2bbb7b91c28dab8b7b56323c29526ab45ff33bbb91009141c4642979fefa8b', 3);
