# Events

Topics: `(contract_symbol, event_symbol, key)`; the key is the pool id, dispute id, or address shown. The indexer depends on this file: any change is breaking.

Short symbols are limited to 9 characters, so some brief names are abbreviated.

| Contract | Event symbol | Key | Data |
|---|---|---|---|
| config | `params` | `updated` | `platform_fee_bp` |
| config | `paused` / `unpaused` | scope | caller / `true` |
| config | `upgraded` | `config` | wasm hash |
| registry | `user_reg` | user | role |
| registry | `user_att` | user | `(role, level)` |
| registry | `user_rev` / `user_sus` | user | `(role, reason)` |
| registry | `user_uns` | user | role |
| reputation | `rep_upd` | subject | `(role, event, tier)` |
| supplier_bond | `deposit` / `withdraw` / `slashed` | supplier | amount |
| group_buy | `pool_new` | pool id | `(organizer, supplier, offer_hash, hub_hash)` |
| group_buy | `committed` / `commit_up` | pool id | `(member, units, amount)` |
| group_buy | `withdrawn` | pool id | `(member, refund)` |
| group_buy | `tier_up` | pool id | `(tier_index, total_units)` |
| group_buy | `filled` | pool id | `(total_units, final_price)` |
| group_buy | `expired` | pool id | total units |
| group_buy | `cancelled` | pool id | caller |
| group_buy | `accepted` / `rejected` | pool id | supplier |
| group_buy | `advance` | pool id | amount |
| group_buy | `dispatch` | pool id | optional waybill hash |
| group_buy | `failed` | pool id | `no_accept` or `no_deliv` |
| group_buy | `delivered` | pool id | `(received_units, evidence_hash)` |
| group_buy | `shortfall` | pool id | missing units |
| group_buy | `alloc_ok` | pool id | received units (shortfall allocation finished; `settle` is now allowed) |
| group_buy | `pickup` | pool id | member |
| group_buy | `settled` | pool id | `(supplier_net, platform_fee, organizer_fee)` |
| group_buy | `refund` | pool id | `(member, amount)` |
| disputes | `d_open` | dispute id | `(pool_id, opener, claimed_amount)` |
| disputes | `d_evid` | dispute id | `(party, evidence_hash)` |
| disputes | `d_resolve` | dispute id | `(arbiter, reasoning_hash)` |
| disputes | `d_timeout` | dispute id | `true` |

Not yet emitted: `tier_changed` (reputation tier changes are carried inside `rep_upd`).
