# 0006 Passkey smart wallet built on OpenZeppelin's WebAuthn verifier

Status: accepted (November 2026). Resolves brief open question 1 for v1. Supersedes the interim wallets in
backend ADR 0003 and frontend ADR 0001.

## Context
Brief 1.4: traders never hold XLM, never see a seed phrase and never need to understand "gas". The interim build
bound a classic G-account whose ed25519 key lived in the browser (IndexedDB) and showed the user a backup secret.
That breaks the brief's promise and leaves the key readable by any script on the page.

Brief 4.1 asks for a passkey-controlled contract account, preferring an existing maintained (ideally audited)
implementation over writing one.

## Options considered
1. **OpenZeppelin `stellar-accounts` smart account (full framework).** Maintained, audited, flexible: context
   rules, policies (thresholds, spending limits), external verifier contracts. For a trader with one phone it
   adds a separate verifier contract per network, per-call context-rule ids in every signature, and an
   `AuthDigestPreimage` the client must build. There is no maintained JS client for it that we could verify, so
   the client side would be custom anyway.
2. **passkey-kit (kalepail).** The reference passkey wallet on Stellar, but not published as a versioned crate
   and not audited.
3. **A minimal wallet whose signature check is OpenZeppelin's audited WebAuthn verifier** (chosen).

## Decision
`contracts/smart_wallet` implements `CustomAccountInterface`:
- signers: up to 10 passkeys (credential id -> uncompressed secp256r1 public key); any one authorises;
  `add_signer` / `remove_signer` need the account's own authorisation; the last signer cannot be removed;
- `__check_auth` looks up the passkey by credential id and calls
  `stellar_accounts::verifiers::webauthn::verify` (v0.8.0), which checks the `webauthn.get` type, that the
  challenge is exactly the Soroban signature payload, user-present and user-verified flags, backup flags, and the
  secp256r1 signature over `sha256(authenticatorData || sha256(clientDataJSON))`;
- not upgradeable: nobody, including SorobanPool, can change the code guarding a user's funds.

The signature payload Soroban hands to `__check_auth` already commits to the network, nonce, expiry ledger and the
whole invocation tree, so an assertion authorises one specific call once. The frontend additionally refuses to
sign an invocation tree it did not ask for (frontend `lib/verify.ts`).

Deployment: the backend sponsor deploys one wallet per user with
`createContractV2(address = sponsor, salt = sha256(credential id), wasm hash, constructor args)`, using the
public key from a WebAuthn registration the backend verified (attestation checked server-side). Only the
sponsor can deploy at those addresses. `scripts/deploy.sh` uploads the wasm and records
`deployments/<network>.json -> smartWallet.wasmHash`.

## Consequences
- The workspace stays on `soroban-sdk =28.0.0` because `stellar-accounts 0.8.0` requires `^28`. Move both
  together when OpenZeppelin publishes a release for 29+.
- Origin and RP ID are not checked on-chain (the platform authenticator binds the credential to the RP ID). A
  passkey for SorobanPool's RP ID can only be exercised on SorobanPool's origin.
- Recovery is "add a second passkey while you still have the first". Account recovery without any passkey
  (lost only phone) is not solved here; it needs a guardian or social-recovery design and is listed in
  `docs/mainnet-checklist.md`.
- Each wallet's instance and signer entries have TTLs. Signer entries are extended on every authorisation; the
  instance (and the shared wallet code) are extended by the backend `ttl-extend` job like the core contracts.
- The wallet is new code holding user funds, so it is in scope for the M6 external audit.
