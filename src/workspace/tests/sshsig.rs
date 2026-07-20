use base64ct::{Base64, Encoding};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256, Sha512};

use crate::domain::ReviewAuthority;
use crate::workspace::sshsig::{parse_authority_key, parse_openssh_public_key, verify};

#[path = "sshsig_malformed.rs"]
mod malformed;

#[test]
fn openssh_key_parser_rejects_each_malformed_boundary() {
    assert!(parse_openssh_public_key(&format!("{}\n", public_key())).is_ok());
    assert!(parse_openssh_public_key("rsa AAAA").is_err());
    assert!(parse_openssh_public_key("ssh-ed25519").is_err());
    assert!(parse_openssh_public_key("ssh-ed25519 !").is_err());
    assert!(parse_openssh_public_key(&format!("{}\n{}", public_key(), public_key())).is_err());
    assert!(
        parse_openssh_public_key(&format!("ssh-ed25519 {}", Base64::encode_string(&[0]))).is_err()
    );
    let mut algorithm_only = Vec::new();
    put_string(&mut algorithm_only, b"ssh-ed25519");
    assert!(
        parse_openssh_public_key(&format!(
            "ssh-ed25519 {}",
            Base64::encode_string(&algorithm_only)
        ))
        .is_err()
    );
    for blob in [
        key_blob(b"ssh-rsa", &[0; 32], false),
        key_blob(b"ssh-ed25519", &[0; 31], false),
        key_blob(b"ssh-ed25519", &[0; 32], true),
    ] {
        assert!(
            parse_openssh_public_key(&format!("ssh-ed25519 {}", Base64::encode_string(&blob)))
                .is_err()
        );
    }
    assert!((0_u32..1024).any(|candidate| {
        let raw: [u8; 32] = Sha256::digest(candidate.to_be_bytes()).into();
        let blob = key_blob(b"ssh-ed25519", &raw, false);
        parse_openssh_public_key(&format!("ssh-ed25519 {}", Base64::encode_string(&blob))).is_err()
    }));
    let mut authority =
        ReviewAuthority::from_openssh("authority".to_owned(), &public_key()).expect("authority");
    authority.fingerprint = "SHA256:wrong".to_owned();
    assert!(parse_authority_key(&authority).is_err());
}

#[test]
fn openssh_key_parser_rejects_weak_ed25519_authorities() {
    let weak_vectors = [
        (
            "order-2 encoding",
            [
                236, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 127,
            ],
        ),
        ("order-4 all-zero encoding", [0; 32]),
    ];

    for (name, weak_key) in weak_vectors {
        let blob = key_blob(b"ssh-ed25519", &weak_key, false);
        let encoded = format!("ssh-ed25519 {}", Base64::encode_string(&blob));

        assert!(parse_openssh_public_key(&encoded).is_err(), "{name}");
    }
}

#[test]
fn sshsig_verifier_rejects_malformed_envelopes_and_accepts_sha256() {
    let key = parse_openssh_public_key(&public_key()).expect("key");
    assert!(verify(&key, "namespace", b"message", "not armor").is_err());
    assert!(
        verify(
            &key,
            "namespace",
            b"message",
            "-----BEGIN SSH SIGNATURE-----\n!\n-----END SSH SIGNATURE-----"
        )
        .is_err()
    );
    assert!(verify(&key, "namespace", b"message", &signed(b"message")).is_ok());
    assert!(
        verify(
            &key,
            "namespace",
            b"message",
            &signed_fields(b"message", b"wrong-namespace", b"")
        )
        .is_err()
    );
    assert!(
        verify(
            &key,
            "namespace",
            b"message",
            &signed_fields(b"message", b"namespace", b"reserved")
        )
        .is_err()
    );
    for signature in malformed::signatures(&key.blob) {
        assert!(verify(&key, "namespace", b"message", &signature).is_err());
    }
}

fn signed(message: &[u8]) -> String {
    signed_fields(message, b"namespace", b"")
}

fn signed_fields(message: &[u8], namespace: &[u8], reserved: &[u8]) -> String {
    let digest = Sha256::digest(message);
    let mut signed = b"SSHSIG".to_vec();
    for field in [namespace, reserved, b"sha256", digest.as_slice()] {
        put_string(&mut signed, field);
    }
    let signature = signing_key().sign(&signed).to_bytes();
    envelope_armor(
        b"SSHSIG",
        1,
        [
            &public_key_blob(),
            namespace,
            reserved,
            b"sha256",
            &signature_blob(b"ssh-ed25519", &signature, false),
        ],
        false,
    )
}

fn envelope_armor(magic: &[u8], version: u32, fields: [&[u8]; 5], trailing: bool) -> String {
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(&version.to_be_bytes());
    for field in fields {
        put_string(&mut bytes, field);
    }
    if trailing {
        bytes.push(0);
    }
    armor(&bytes)
}

fn signature_blob(algorithm: &[u8], signature: &[u8], trailing: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    put_string(&mut bytes, algorithm);
    put_string(&mut bytes, signature);
    if trailing {
        bytes.push(0);
    }
    bytes
}

fn key_blob(algorithm: &[u8], key: &[u8], trailing: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    put_string(&mut bytes, algorithm);
    put_string(&mut bytes, key);
    if trailing {
        bytes.push(0);
    }
    bytes
}

fn signing_key() -> SigningKey {
    let seed: [u8; 32] = Sha512::digest(b"sshsig parser test")[..32]
        .try_into()
        .expect("seed");
    SigningKey::from_bytes(&seed)
}

fn public_key_blob() -> Vec<u8> {
    key_blob(
        b"ssh-ed25519",
        signing_key().verifying_key().as_bytes(),
        false,
    )
}

fn public_key() -> String {
    format!("ssh-ed25519 {}", Base64::encode_string(&public_key_blob()))
}

fn put_string(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value);
}

fn armor(bytes: &[u8]) -> String {
    format!(
        "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----",
        Base64::encode_string(bytes)
    )
}
