use base64ct::{Base64, Base64Unpadded, Encoding};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256, Sha512};

use crate::domain::ReviewAuthority;
use crate::{Error, Result};

pub(super) struct ParsedPublicKey {
    pub(super) blob: Vec<u8>,
    key: VerifyingKey,
}

pub(super) fn parse_authority_key(authority: &ReviewAuthority) -> Result<ParsedPublicKey> {
    let parsed = parse_openssh_public_key(&authority.public_key)?;
    let fingerprint = fingerprint(&parsed);
    if fingerprint != authority.fingerprint {
        return Err(Error::invalid(
            "review authority",
            "public key and fingerprint do not match",
        ));
    }
    Ok(parsed)
}

pub(super) fn parse_openssh_public_key(encoded: &str) -> Result<ParsedPublicKey> {
    let encoded = encoded.strip_suffix('\n').unwrap_or(encoded);
    if encoded.is_empty() || encoded.contains('\r') || encoded.contains('\n') {
        return Err(Error::invalid(
            "review authority public_key",
            "exactly one nonempty OpenSSH key record is required",
        ));
    }
    let mut fields = encoded.split_whitespace();
    if fields.next() != Some("ssh-ed25519") {
        return Err(Error::invalid(
            "review authority public_key",
            "only OpenSSH ssh-ed25519 keys are supported",
        ));
    }
    let encoded_blob = fields.next().ok_or_else(|| {
        Error::invalid(
            "review authority public_key",
            "OpenSSH key payload is missing",
        )
    })?;
    let blob = Base64::decode_vec(encoded_blob).map_err(|_| {
        Error::invalid(
            "review authority public_key",
            "OpenSSH key payload is malformed",
        )
    })?;
    let mut reader = SshReader::new(&blob);
    if reader.string()? != b"ssh-ed25519" {
        return Err(Error::invalid(
            "review authority public_key",
            "key payload algorithm is not ssh-ed25519",
        ));
    }
    let raw_key: [u8; 32] = reader.string()?.try_into().map_err(|_| {
        Error::invalid(
            "review authority public_key",
            "Ed25519 key must be 32 bytes",
        )
    })?;
    reader.finish("review authority public_key")?;
    let key = VerifyingKey::from_bytes(&raw_key)
        .map_err(|_| Error::invalid("review authority public_key", "Ed25519 key is invalid"))?;
    Ok(ParsedPublicKey { blob, key })
}

pub(super) fn fingerprint(key: &ParsedPublicKey) -> String {
    format!(
        "SHA256:{}",
        Base64Unpadded::encode_string(&Sha256::digest(&key.blob))
    )
}

pub(super) fn verify(
    authority: &ParsedPublicKey,
    expected_namespace: &str,
    message: &[u8],
    armored: &str,
) -> Result<()> {
    let bytes = decode_armor(armored)?;
    let mut reader = SshReader::new(&bytes);
    if reader.take(6)? != b"SSHSIG" || reader.u32()? != 1 {
        return Err(Error::invalid(
            "review authorization",
            "signature is not SSHSIG version 1",
        ));
    }
    if reader.string()? != authority.blob {
        return Err(Error::invalid(
            "review authorization",
            "signature key does not match the enrolled authority",
        ));
    }
    let namespace = reader.string()?;
    let reserved = reader.string()?;
    let hash_algorithm = reader.string()?;
    let signature_blob = reader.string()?;
    reader.finish("review authorization")?;
    if namespace != expected_namespace.as_bytes() || !reserved.is_empty() {
        return Err(Error::invalid(
            "review authorization",
            "signature namespace or reserved field is invalid",
        ));
    }
    verify_signature(
        authority,
        namespace,
        reserved,
        hash_algorithm,
        signature_blob,
        message,
    )
}

fn verify_signature(
    authority: &ParsedPublicKey,
    namespace: &[u8],
    reserved: &[u8],
    hash_algorithm: &[u8],
    signature_blob: &[u8],
    message: &[u8],
) -> Result<()> {
    let digest = match hash_algorithm {
        b"sha256" => Sha256::digest(message).to_vec(),
        b"sha512" => Sha512::digest(message).to_vec(),
        _ => {
            return Err(Error::invalid(
                "review authorization",
                "signature hash algorithm must be sha256 or sha512",
            ));
        }
    };
    let mut signed = b"SSHSIG".to_vec();
    put_string(&mut signed, namespace);
    put_string(&mut signed, reserved);
    put_string(&mut signed, hash_algorithm);
    put_string(&mut signed, &digest);
    let mut signature_reader = SshReader::new(signature_blob);
    if signature_reader.string()? != b"ssh-ed25519" {
        return Err(Error::invalid(
            "review authorization",
            "signature algorithm must be ssh-ed25519",
        ));
    }
    let signature = Signature::from_slice(signature_reader.string()?)
        .map_err(|_| Error::invalid("review authorization", "Ed25519 signature is malformed"))?;
    signature_reader.finish("review authorization")?;
    authority.key.verify(&signed, &signature).map_err(|_| {
        Error::invalid(
            "review authorization",
            "signature does not authorize this exact review request",
        )
    })
}

fn decode_armor(armored: &str) -> Result<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN SSH SIGNATURE-----";
    const END: &str = "-----END SSH SIGNATURE-----";
    let body = armored
        .trim()
        .strip_prefix(BEGIN)
        .and_then(|value| value.strip_suffix(END))
        .ok_or_else(|| Error::invalid("review authorization", "signature is not armored SSHSIG"))?;
    let encoded: String = body
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    Base64::decode_vec(&encoded)
        .map_err(|_| Error::invalid("review authorization", "SSHSIG armor is malformed"))
}

fn put_string(output: &mut Vec<u8>, value: &[u8]) {
    let length = value.len() as u32;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
}

struct SshReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> SshReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self.position.saturating_add(length);
        let value = self.bytes.get(self.position..end).ok_or_else(|| {
            Error::invalid("SSH encoding", "field extends beyond the available input")
        })?;
        self.position = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn string(&mut self) -> Result<&'a [u8]> {
        let length = self.u32()? as usize;
        self.take(length)
    }

    fn finish(&self, field: &str) -> Result<()> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::invalid(field, "SSH encoding contains trailing data"))
        }
    }
}
