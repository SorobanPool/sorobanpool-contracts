# Error codes

Generated from the `Error` enums in each contract. Codes must stay within the
contract namespace below; `scripts/export_artifacts.py` fails on duplicates or
out-of-range values, and `scripts/test_error_ranges.py` tests both failure
modes.

| Contract | Reserved range |
|---|---:|
| config | 100–199 |
| registry | 200–299 |
| group_buy | 300–399 |
| disputes | 400–499 |
| supplier_bond | 500–599 |
| reputation | 600–699 |
| smart_wallet | 700–799 |

| Code | Contract | Name |
|---|---|---|
| 100 | config | NotAdmin |
| 101 | config | NotAdminOrGuardian |
| 102 | config | ParamsInvalid |
| 103 | config | ParamsExceedHardCap |
| 104 | config | AddressNotSet |
| 105 | config | NoGuardian |
| 200 | registry | AlreadyRegistered |
| 201 | registry | NotRegistered |
| 202 | registry | NotAttestor |
| 203 | registry | NotAdmin |
| 204 | registry | NotAuthorizedCaller |
| 205 | registry | InvalidLevel |
| 206 | registry | BadStatus |
| 300 | group_buy | NotInitialised |
| 301 | group_buy | Paused |
| 302 | group_buy | BadTerms |
| 303 | group_buy | CategoryNotAllowed |
| 304 | group_buy | NotVerified |
| 305 | group_buy | DeadlineInPast |
| 346 | group_buy | DeadlineTooFar |
| 349 | group_buy | ZeroPageSize |
| 306 | group_buy | FeeTooHigh |
| 307 | group_buy | PoolTooLarge |
| 308 | group_buy | PoolNotFound |
| 309 | group_buy | WrongState |
| 310 | group_buy | DeadlinePassed |
| 311 | group_buy | DeadlineNotReached |
| 312 | group_buy | ZeroUnits |
| 313 | group_buy | NotAllowedMember |
| 314 | group_buy | ClusterMismatch |
| 315 | group_buy | OverMemberLimit |
| 316 | group_buy | OverMemberShare |
| 317 | group_buy | OverPoolMax |
| 318 | group_buy | OverTierCap |
| 319 | group_buy | TooManyMembers |
| 320 | group_buy | AlreadyCommitted |
| 321 | group_buy | NotCommitted |
| 322 | group_buy | WithdrawLocked |
| 323 | group_buy | WouldLowerTier |
| 324 | group_buy | BelowMoq |
| 325 | group_buy | NotOrganizer |
| 326 | group_buy | NotSupplier |
| 327 | group_buy | NotAdmin |
| 328 | group_buy | CannotCancel |
| 329 | group_buy | AcceptWindowPassed |
| 330 | group_buy | AcceptWindowOpen |
| 331 | group_buy | AdvanceDisabled |
| 332 | group_buy | AdvanceTooHigh |
| 333 | group_buy | DeliveryDeadlineNotPassed |
| 334 | group_buy | BadReceivedUnits |
| 335 | group_buy | ConfirmDeadlineNotPassed |
| 336 | group_buy | AlreadyConfirmed |
| 337 | group_buy | AllocationPending |
| 338 | group_buy | SettleTooEarly |
| 339 | group_buy | NothingToClaim |
| 340 | group_buy | NotDisputes |
| 341 | group_buy | FreezeTooLarge |
| 342 | group_buy | DisputeWindowClosed |
| 343 | group_buy | Overflow |
| 344 | group_buy | NotFinal |
| 345 | group_buy | SupplierIsParty |
| 347 | group_buy | OrganizerIsSupplier |
| 348 | group_buy | InvalidRejectReason |
| 400 | disputes | NotInitialised |
| 401 | disputes | NotAParty |
| 402 | disputes | BadClaim |
| 403 | disputes | NotFound |
| 404 | disputes | AlreadyResolved |
| 405 | disputes | NotArbiter |
| 406 | disputes | ArbiterConflict |
| 407 | disputes | SlaPassed |
| 408 | disputes | SlaNotReached |
| 409 | disputes | BadOutcome |
| 410 | disputes | TooMuchEvidence |
| 411 | disputes | Overflow |
| 412 | disputes | AlreadyDisputed |
| 413 | disputes | TooManyOpenDisputes |
| 500 | supplier_bond | InvalidAmount |
| 501 | supplier_bond | InsufficientFreeBond |
| 502 | supplier_bond | NotGroupBuy |
| 503 | supplier_bond | NotInitialised |
| 504 | supplier_bond | ReleaseExceedsReserved |
| 505 | supplier_bond | Overflow |
| 600 | reputation | NotAuthorizedCaller |
| 601 | reputation | UnknownEvent |
| 602 | reputation | NotInitialised |
| 700 | smart_wallet | UnknownSigner |
| 701 | smart_wallet | SignerExists |
| 702 | smart_wallet | LastSigner |
| 703 | smart_wallet | TooManySigners |
| 704 | smart_wallet | BadCredentialId |
| 705 | smart_wallet | SignerNotActive |

Disputes accept at most 20 evidence hashes, including the opener's initial hash.
The 21st hash fails with `TooMuchEvidence` (410); evidence additions after
resolution or timeout fail with `EvidenceClosed` (414). `BadOutcome` (409) is
returned when the arbiter chooses `RefundMember(units)` above the member's
allocation or the claimed units, or `Split(bp)` above 10,000 bp; the dispute
stays open. A dispute deposit is capped at the claimed amount so a tiny claim
never requires a larger deposit.

## Exported Artifact Schema

`artifacts/errors.json` is exported by `scripts/export_artifacts.py` with top-level metadata:

```json
{
  "schemaVersion": 1,
  "contractsVersion": "0.1.0",
  "errors": [
    {
      "code": 100,
      "contract": "config",
      "name": "NotAdmin"
    }
  ]
}
```

Consumers (e.g. frontend error mapping, backend indexers) use `schemaVersion` and `contractsVersion` to detect incompatible versions.
