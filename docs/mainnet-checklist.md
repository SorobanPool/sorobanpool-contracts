# Mainnet readiness checklist

What is done, what is left, and who owns the parts this repository cannot finish by itself. Status as of this testnet build across all three repos (contracts, backend, frontend).

## Contracts
- [x] Six contracts plus shared pricing/escrow logic; 62 tests, 600 cross-repo pricing vectors, a conservation property test, randomised sequence fuzzing.
- [x] ≥90% line coverage on `group_buy`, `disputes`, `supplier_bond`, enforced in CI.
- [x] Resource budget tests at a 200-member worst case (commit/settle/claim_refund <50%, push_refunds(25) <80% of an assumed limit).
- [ ] **External audit.** Not something this repository can do; needs a hired firm. `scripts/deploy.sh` already refuses a mainnet deploy without `MAINNET_CONFIRM=yes`, a clean tree, a release tag and an audit hash — the hash itself still needs to come from a real audit.
- [x] Budget test limits are the real testnet network config (not assumed): `txMaxInstructions`=400,000,000, `txMemoryLimit`=41,943,040 bytes, read live via RPC. Still: native execution under-reports Wasm cost, and mainnet config may differ from testnet's — re-confirm there before relying on this.
- [ ] Coverage-guided fuzzing (cargo-fuzz, needs nightly Rust) as a follow-up to the property-based fuzzing already in place.
- [ ] Admin key: must be a real multisig before mainnet (see threat model).

## Backend
- [x] API, indexer, keeper jobs (close/fail/allocate/settle/push-refunds/dispute-timeout), ttl-extend, maintenance (offer expiry, sponsor-balance alert, deadline reminders, reconcile), notifier (SMS queue, quiet hours, retries).
- [x] Rate limiting, Prometheus metrics, generated OpenAPI, S3-compatible storage adapter, passkey sign-in, Sentry error reporting, CI run against real PostgreSQL 16.
- [ ] **Real naira anchor and withdrawals.** Only a mock exists (ADR 0005). Needs a licensed rail and a business decision on the provider.
- [ ] **Live FX source.** Only static rates exist; no `fx-refresh` job. Needs at least two independent quote sources.
- [ ] **Real SMS/WhatsApp and KYC/KYB providers.** The `SmsSender` port exists (console sender only); KYC/KYB is unimplemented. Needs provider contracts.
- [ ] BullMQ (ADR 0002 chose an in-process scheduler for now) — run exactly one worker replica until this moves to a real queue, or multiple replicas will double-send keeper calls and notifications.
- [ ] OpenTelemetry, a pager integration (alerts are currently log lines), Testcontainers for Redis/MinIO.
- [ ] S3 adapter and Sentry delivery are tested against local fakes, not the real providers — confirm against the actual provider before relying on them in production.

## Frontend
- [x] Full PWA (trader/organizer, supplier, disputes, arbiter, admin), EN + Pidgin, live pool updates (SSE), passkey sign-in, offline support, accessibility (axe, EN/PCM/admin).
- [x] CI-enforced JS size budget on public pages (Lighthouse).
- [ ] **Pidgin copy review by a native speaker.** Written by this build, unreviewed.
- [ ] **Passkey-controlled smart wallet** (ADR 0001) is a separate, bigger decision from the passkey sign-in shipped here (sign-in only, wallet unchanged).
- [ ] Native-device testing — only Playwright's mobile emulation has run.
- [ ] Lighthouse performance/LCP thresholds are measured but not gated in CI (only JS size is); add once a stable CI runner baseline is established.

## Process
- [ ] Mainnet pilot plan and go/no-go criteria (brief milestone M7) — a product and business decision, not an engineering one.
- [ ] Legal/compliance review for handling Nigerian naira payments and KYC data.

This file should be updated as items move from the "left" lists to the "done" lists; it is not meant to be regenerated.
