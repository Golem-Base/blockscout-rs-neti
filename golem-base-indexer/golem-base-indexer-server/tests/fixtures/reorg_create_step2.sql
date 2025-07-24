update blocks set consensus = false where number = 2;
INSERT INTO public.blocks (consensus, difficulty, gas_limit, gas_used, hash, miner_hash, nonce, number, parent_hash, size, "timestamp", total_difficulty, inserted_at, updated_at, refetch_needed, base_fee_per_gas, is_empty) VALUES (true, 0, 11522469, 1022440, '\x8145c1f3c0b925535ff1425ec1f4f5aa80206a91c4b60b35392cba3a673a8a3a', '\x48108710b260831e47b7cc552dec5f8b12cca123', '\x0000000000000000', 2, '\x7c005134bb8bdb6bad330b6627041f89225417fd828d2fb8715309a1b01308c2', 877, '2025-07-24 08:51:56', NULL, '2025-07-24 08:52:02.13342', '2025-07-24 08:52:02.13342', false, 785027253, NULL);


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
where hash = '\xd50097b0a75a8b254407ece5be421a332f50f7b640b870f745cc83266aed1703';

INSERT INTO public.transactions (cumulative_gas_used, error, gas, gas_price, gas_used, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, created_contract_address_hash, created_contract_code_indexed_at, earliest_processing_start, old_block_hash, revert_reason, max_priority_fee_per_gas, max_fee_per_gas, type, has_error_in_internal_transactions, block_timestamp, block_consensus) VALUES (1022440, NULL, 22775, 785027254, 22440, '\xa2be32cb84f0aea1d409c785176292053e6e02208574ba81fe4d07f5459abc43', 1, '\xe3dfde649a746869732077696c6c20737461792061667465722072656f7267c0c0c0c0c0', 0, 111788057661671684872429544898557613159089551078437322986240854960110134479422, 32474820977468205673464369374730722286065360965279987205247030824286789124084, 1, 0, 0, '2025-07-24 08:52:02.13342', '2025-07-24 08:52:02.791677', '\x8145c1f3c0b925535ff1425ec1f4f5aa80206a91c4b60b35392cba3a673a8a3a', 2, '\xd29bb1a1a0f6d2783306a8618b3a5b58cb313152', '\x0000000000000000000000000000000060138453', NULL, NULL, NULL, NULL, NULL, 1, 1750000001, 2, false, '2025-07-24 08:51:56', true);

INSERT INTO public.logs (data, index, first_topic, second_topic, third_topic, fourth_topic, inserted_at, updated_at, address_hash, transaction_hash, block_hash, block_number) VALUES ('\x0000000000000000000000000000000000000000000000000000000000000066', 0, '\xce4b4ad6891d716d0b1fba2b4aeb05ec20edadb01df512263d0dde423736bbb9', '\x2d8eeaf460fddbc21ab54560edfa5db27bf24914264fe9a61265d5d93e41ce5c', NULL, NULL, '2025-07-24 08:52:02.13342', '2025-07-24 08:52:02.13342', '\x0000000000000000000000000000000060138453', '\xa2be32cb84f0aea1d409c785176292053e6e02208574ba81fe4d07f5459abc43', '\x8145c1f3c0b925535ff1425ec1f4f5aa80206a91c4b60b35392cba3a673a8a3a', 2);
