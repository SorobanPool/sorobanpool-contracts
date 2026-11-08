extern crate std;

use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use p256::ecdsa::{signature::Signer, Signature as P256Sig, SigningKey};
use soroban_sdk::{testutils::BytesN as _, vec, Env, IntoVal};
use std::{format, vec::Vec as StdVec};

const UP_UV: u8 = 0x05;

struct Passkey {
    id: Bytes,
    sk: SigningKey,
}

impl Passkey {
    fn new(env: &Env, seed: u8) -> Self {
        let sk = SigningKey::from_slice(&[seed; 32]).unwrap();
        Passkey {
            id: Bytes::from_array(env, &[seed; 20]),
            sk,
        }
    }

    fn public_key(&self, env: &Env) -> BytesN<65> {
        let point = self.sk.verifying_key().to_encoded_point(false);
        BytesN::from_array(env, point.as_bytes().try_into().unwrap())
    }

    /// What a browser returns from navigator.credentials.get for `payload`, packaged as the contract expects.
    fn assert(&self, env: &Env, payload: &[u8; 32], flags: u8, kind: &str) -> Signature {
        let challenge = URL_SAFE_NO_PAD.encode(payload);
        let client = format!(
            r#"{{"type":"{kind}","challenge":"{challenge}","origin":"https://sorobanpool.app","crossOrigin":false}}"#
        );
        let mut auth_data = StdVec::from([0x11u8; 32]); // rpIdHash
        auth_data.push(flags);
        auth_data.extend_from_slice(&[0, 0, 0, 1]); // sign counter
        let client_hash = sha2_256(client.as_bytes());
        let mut msg = auth_data.clone();
        msg.extend_from_slice(&client_hash);
        // ES256: ECDSA over SHA-256(msg). The host rejects high-S, as browsers may produce; clients normalise.
        let sig: P256Sig = self.sk.sign(&msg);
        let sig = sig.normalize_s().unwrap_or(sig);
        Signature {
            credential_id: self.id.clone(),
            authenticator_data: Bytes::from_slice(env, &auth_data),
            client_data: Bytes::from_slice(env, client.as_bytes()),
            signature: BytesN::from_array(env, &sig.to_bytes().into()),
        }
    }
}

fn sha2_256(data: &[u8]) -> [u8; 32] {
    let env = Env::default();
    env.crypto()
        .sha256(&Bytes::from_slice(&env, data))
        .to_array()
}

fn setup() -> (Env, Address, Passkey) {
    let env = Env::default();
    let pk = Passkey::new(&env, 7);
    let id = env.register(SmartWallet, (pk.id.clone(), pk.public_key(&env)));
    (env, id, pk)
}

fn check(env: &Env, wallet: &Address, payload: &BytesN<32>, sig: Signature) -> Result<(), Error> {
    env.try_invoke_contract_check_auth::<Error>(wallet, payload, sig.into_val(env), &vec![env])
        .map_err(|e| match e {
            Ok(err) => err,
            Err(_) => Error::UnknownSigner, // host-level failure (bad signature, failed WebAuthn check)
        })
}

fn host_rejects(env: &Env, wallet: &Address, payload: &BytesN<32>, sig: Signature) -> bool {
    env.try_invoke_contract_check_auth::<Error>(wallet, payload, sig.into_val(env), &vec![env])
        .is_err()
}

use soroban_sdk::Address;

#[test]
fn a_valid_passkey_assertion_authorises() {
    let (env, wallet, pk) = setup();
    let payload = BytesN::<32>::random(&env);
    let sig = pk.assert(&env, &payload.to_array(), UP_UV, "webauthn.get");
    assert_eq!(check(&env, &wallet, &payload, sig), Ok(()));
}

#[test]
fn an_assertion_for_a_different_payload_is_rejected() {
    let (env, wallet, pk) = setup();
    let payload = BytesN::<32>::random(&env);
    let other = BytesN::<32>::random(&env);
    let sig = pk.assert(&env, &other.to_array(), UP_UV, "webauthn.get");
    assert!(host_rejects(&env, &wallet, &payload, sig));
}

#[test]
fn a_signature_from_an_unregistered_key_is_rejected() {
    let (env, wallet, pk) = setup();
    let payload = BytesN::<32>::random(&env);
    let attacker = Passkey::new(&env, 9);
    // Claims the victim's credential id but signs with another key.
    let mut sig = attacker.assert(&env, &payload.to_array(), UP_UV, "webauthn.get");
    sig.credential_id = pk.id.clone();
    assert!(host_rejects(&env, &wallet, &payload, sig));
    // Unknown credential id altogether.
    let sig = attacker.assert(&env, &payload.to_array(), UP_UV, "webauthn.get");
    assert_eq!(
        check(&env, &wallet, &payload, sig),
        Err(Error::UnknownSigner)
    );
}

#[test]
fn user_verification_is_required() {
    let (env, wallet, pk) = setup();
    let payload = BytesN::<32>::random(&env);
    let present_only = pk.assert(&env, &payload.to_array(), 0x01, "webauthn.get");
    assert!(host_rejects(&env, &wallet, &payload, present_only));
}

#[test]
fn a_registration_ceremony_cannot_be_replayed_as_an_assertion() {
    let (env, wallet, pk) = setup();
    let payload = BytesN::<32>::random(&env);
    let create = pk.assert(&env, &payload.to_array(), UP_UV, "webauthn.create");
    assert!(host_rejects(&env, &wallet, &payload, create));
}

#[test]
fn tampered_authenticator_data_is_rejected() {
    let (env, wallet, pk) = setup();
    let payload = BytesN::<32>::random(&env);
    let mut sig = pk.assert(&env, &payload.to_array(), UP_UV, "webauthn.get");
    sig.authenticator_data.set(36, 9); // bump the counter after signing
    assert!(host_rejects(&env, &wallet, &payload, sig));
}

#[test]
fn signers_can_be_added_and_removed_but_never_the_last() {
    let (env, wallet, pk) = setup();
    let c = SmartWalletClient::new(&env, &wallet);
    assert_eq!(c.signer_count(), 1);
    assert_eq!(c.public_key(&pk.id), Some(pk.public_key(&env)));

    // Management calls need the account's own authorisation.
    let phone2 = Passkey::new(&env, 8);
    assert!(c
        .try_add_signer(&phone2.id, &phone2.public_key(&env))
        .is_err());

    env.mock_all_auths();
    c.add_signer(&phone2.id, &phone2.public_key(&env));
    assert_eq!(c.signer_count(), 2);
    assert!(c
        .try_add_signer(&phone2.id, &phone2.public_key(&env))
        .is_err()); // duplicate
    let payload = BytesN::<32>::random(&env);
    let sig = phone2.assert(&env, &payload.to_array(), UP_UV, "webauthn.get");
    assert_eq!(check(&env, &wallet, &payload, sig), Ok(()));

    c.remove_signer(&pk.id);
    assert_eq!(c.signer_count(), 1);
    assert_eq!(c.public_key(&pk.id), None);
    assert_eq!(
        c.try_remove_signer(&phone2.id),
        Err(Ok(Error::LastSigner.into()))
    );
    assert_eq!(
        c.try_remove_signer(&pk.id),
        Err(Ok(Error::UnknownSigner.into()))
    );
}

#[test]
fn signer_limits() {
    let (env, wallet, _pk) = setup();
    let c = SmartWalletClient::new(&env, &wallet);
    env.mock_all_auths();
    for seed in 10..(10 + MAX_SIGNERS as u8 - 1) {
        let p = Passkey::new(&env, seed);
        c.add_signer(&p.id, &p.public_key(&env));
    }
    let extra = Passkey::new(&env, 99);
    assert_eq!(
        c.try_add_signer(&extra.id, &extra.public_key(&env)),
        Err(Ok(Error::TooManySigners.into()))
    );
    assert_eq!(
        c.try_remove_signer(&Bytes::new(&env)),
        Err(Ok(Error::BadCredentialId.into()))
    );
}
