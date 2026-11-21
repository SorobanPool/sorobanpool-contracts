# ADR 0007: Closed roles may re-register

An admin can close a registry role. The same address may then register again for
that role with a fresh profile hash. The new record starts unverified; reputation
is stored by the separate reputation contract and is intentionally not reset.

