# Events

Topics: `(contract_symbol, event_symbol, key)`; the key is the pool id, dispute id, or address shown. The indexer depends on this file: any change is breaking.

Short symbols are limited to 9 characters, so some brief names are abbreviated. The *contract* symbol is therefore `rep` for reputation and `bond` for supplier_bond (not their crate names). `scripts/export_artifacts.py` fails CI if this table and the emitted events ever disagree, in either direction.

| Contract | Event symbol | Key | Data |
|---|---|---|---|
| config | `params` | `updated` | `platform_fee_bp` |
| config | `paused` / `unpaused` | scope | caller / `true` |
| config | `upgraded` | `config` | wasm hash |
| config | `admin_proposed` / `admin_changed` | new admin | caller / old admin |
| registry | `user_reg` | user | role |
| registry | `user_att` | user | `(role, level)` |
| registry | `user_rev` / `user_sus` / `user_close` | user | `(role, reason)` |
| registry | `user_uns` | user | role |
| rep | `rep_upd` | subject | `(role, event, tier)` |
| rep | `rep_corrected` | subject | `(role, counter, delta, reason)` |
| bond | `deposit` / `withdraw` | supplier | amount |
| bond | `slashed` | supplier | `(requested, slashed)` |
| group_buy | `pool_new` | pool id | `(organizer, supplier, offer_hash, hub_hash)` |
| group_buy | `committed` / `commit_up` | pool id | `(member, units, amount)` |
| group_buy | `withdrawn` | pool id | `(member, refund)` |
| group_buy | `tier_up` | pool id | `(tier_index, total_units)` |
| group_buy | `filled` | pool id | `(total_units, final_price)` |
| group_buy | `expired` | pool id | total units |
| group_buy | `cancelled` | pool id | caller |
| group_buy | `accepted` | pool id | supplier |
| group_buy | `rejected` | pool id | `(supplier, reason)` |
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
| wallet | `key_add` / `key_rm` | credential id | `()` (emitted by each user's passkey wallet, not by a SorobanPool core contract; the indexer does not watch wallets) |

Not yet emitted: `tier_changed` (reputation tier changes are carried inside `rep_upd`).

## Event Rules

**tier_up idempotency:** The `tier_up` event is emitted at most once per (pool_id, tier_index) pair. If a pool crosses from tier 1 to tier 2 (via commitment), then drops below tier 2 (via withdrawal), and later crosses tier 2 again (via re-commitment), the second crossing does not emit another event. Indexers can rely on each tier milestone being reported exactly once per pool.

## Exported Artifact Schema

`artifacts/events.json` is exported by `scripts/export_artifacts.py` with top-level metadata:

```json
{
  "schemaVersion": 1,
  "contractsVersion": "0.1.0",
  "events": [
    {
      "contract": "config",
      "event": "params",
      "key": "updated",
      "data": "platform_fee_bp"
    }
  ]
}
```

Consumers (e.g. backend event processors and indexers) verify `schemaVersion` and `contractsVersion` against deployed contract builds.
