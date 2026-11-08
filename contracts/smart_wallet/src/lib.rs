//! Passkey-controlled account (brief 4.1, open question 1; ADR 0006).
//!
//! A user's on-chain identity is this contract. Its signers are WebAuthn passkeys (secp256r1 / ES256): the
//! private key never leaves the phone's secure hardware and the user never sees a seed phrase. Any registered
//! passkey can authorise any action for the account, like a normal wallet; a user adds a second passkey (a new
//! phone) with a signature from an existing one.
//!
//! Signature verification is OpenZeppelin's audited `stellar_accounts::verifiers::webauthn::verify`: it checks
//! the `webauthn.get` type, that the challenge is exactly the Soroban signature payload, the user-present and
//! user-verified flags (fingerprint, face or PIN), the backup flags and the secp256r1 signature (low-S, enforced
//! by the host). Origin and RP ID are not checked on-chain; the platform authenticator enforces them.
//!
//! The contract is not upgradeable: nobody, including SorobanPool, can change the code that guards a user's
//! funds. Fees are paid by the SorobanPool sponsor, never by the account.
#![no_std]
use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    panic_with_error, symbol_short, Bytes, BytesN, Env, Symbol, Vec,
};
use sp_common::events::emit;
use stellar_accounts::verifiers::webauthn::{self, WebAuthnSigData};

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    UnknownSigner = 700,
    SignerExists = 701,
    LastSigner = 702,
    TooManySigners = 703,
    BadCredentialId = 704,
}

/// What a client puts in the authorisation entry's signature: which passkey signed, and its assertion.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    /// WebAuthn credential id (raw bytes) of the signing passkey.
    pub credential_id: Bytes,
    pub authenticator_data: Bytes,
    pub client_data: Bytes,
    /// r || s, 32 bytes each, s in the lower half of the curve order.
    pub signature: BytesN<64>,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    /// credential id -> uncompressed secp256r1 public key (0x04 || x || y).
    Signer(Bytes),
    Count,
}

/// A handful of devices is plenty for a trader; the cap keeps storage and review simple.
pub const MAX_SIGNERS: u32 = 10;
/// Credential ids are at most 1023 bytes by spec; real platform authenticators use 16 to 64.
const MAX_CREDENTIAL_ID: u32 = 255;
const C: Symbol = symbol_short!("wallet");

// Persistent signer entries: extended on every authorisation (~30 days left -> ~120 days).
const DAY_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 30 * DAY_LEDGERS;
const TTL_EXTEND_TO: u32 = 120 * DAY_LEDGERS;

fn signer_key(env: &Env, credential_id: &Bytes) -> Key {
    if credential_id.is_empty() || credential_id.len() > MAX_CREDENTIAL_ID {
        panic_with_error!(env, Error::BadCredentialId);
    }
    Key::Signer(credential_id.clone())
}

fn count(env: &Env) -> u32 {
    env.storage().instance().get(&Key::Count).unwrap_or(0)
}

fn put_signer(env: &Env, credential_id: &Bytes, public_key: &BytesN<65>) {
    let k = signer_key(env, credential_id);
    if env.storage().persistent().has(&k) {
        panic_with_error!(env, Error::SignerExists);
    }
    let n = count(env);
    if n >= MAX_SIGNERS {
        panic_with_error!(env, Error::TooManySigners);
    }
    env.storage().persistent().set(&k, public_key);
    env.storage()
        .persistent()
        .extend_ttl(&k, TTL_THRESHOLD, TTL_EXTEND_TO);
    env.storage().instance().set(&Key::Count, &(n + 1));
    emit(env, C, symbol_short!("key_add"), credential_id.clone(), ());
}

#[contract]
pub struct SmartWallet;

#[contractimpl]
impl SmartWallet {
    /// Deployed by the SorobanPool sponsor with the user's first passkey (verified server-side at registration).
    pub fn __constructor(env: Env, credential_id: Bytes, public_key: BytesN<65>) {
        put_signer(&env, &credential_id, &public_key);
    }

    /// Adds another passkey (e.g. a new phone). Must be authorised by an existing passkey.
    pub fn add_signer(env: Env, credential_id: Bytes, public_key: BytesN<65>) {
        env.current_contract_address().require_auth();
        put_signer(&env, &credential_id, &public_key);
    }

    /// Removes a passkey (lost phone). The last passkey can never be removed, so the account cannot lock itself.
    pub fn remove_signer(env: Env, credential_id: Bytes) {
        env.current_contract_address().require_auth();
        let k = signer_key(&env, &credential_id);
        if !env.storage().persistent().has(&k) {
            panic_with_error!(&env, Error::UnknownSigner);
        }
        let n = count(&env);
        if n <= 1 {
            panic_with_error!(&env, Error::LastSigner);
        }
        env.storage().persistent().remove(&k);
        env.storage().instance().set(&Key::Count, &(n - 1));
        emit(&env, C, symbol_short!("key_rm"), credential_id, ());
    }

    pub fn public_key(env: Env, credential_id: Bytes) -> Option<BytesN<65>> {
        env.storage()
            .persistent()
            .get(&signer_key(&env, &credential_id))
    }

    pub fn signer_count(env: Env) -> u32 {
        count(&env)
    }
}

#[contractimpl]
impl CustomAccountInterface for SmartWallet {
    type Signature = Signature;
    type Error = Error;

    /// Called by the host for every `require_auth` on this account. The signature payload already commits to
    /// the network, the nonce, the expiry ledger and the full invocation tree, so a valid passkey assertion over
    /// it authorises exactly that call and nothing else, once.
    #[allow(non_snake_case)]
    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signature: Signature,
        _auth_contexts: Vec<Context>,
    ) -> Result<(), Error> {
        let k = signer_key(&env, &signature.credential_id);
        let public_key: BytesN<65> = env
            .storage()
            .persistent()
            .get(&k)
            .ok_or(Error::UnknownSigner)?;
        env.storage()
            .persistent()
            .extend_ttl(&k, TTL_THRESHOLD, TTL_EXTEND_TO);
        let payload = Bytes::from_array(&env, &signature_payload.to_array());
        let sig = WebAuthnSigData {
            signature: signature.signature,
            authenticator_data: signature.authenticator_data,
            client_data: signature.client_data,
        };
        // Panics (rejecting the authorisation) on any failed check.
        webauthn::verify(&env, &payload, &public_key, &sig);
        Ok(())
    }
}

#[cfg(test)]
mod test;
