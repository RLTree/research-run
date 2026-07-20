use super::*;

pub(super) fn signatures(key: &[u8]) -> Vec<String> {
    let mut signatures = malformed_headers(key);
    signatures.extend(truncated_envelope_fields(key));
    signatures.extend(malformed_envelope_payloads(key));
    signatures.extend(malformed_signature_payloads(key));
    signatures
}

fn truncated_envelope_fields(key: &[u8]) -> Vec<String> {
    let mut bytes = b"SSHSIG".to_vec();
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    let mut signatures = vec![armor(&bytes)];
    for field in [key, b"namespace", b"", b"sha512"] {
        put_string(&mut bytes, field);
        signatures.push(armor(&bytes));
    }
    signatures
}

fn malformed_headers(key: &[u8]) -> Vec<String> {
    let signature = signature_blob(b"ssh-ed25519", &[0; 64], false);
    vec![
        armor(b"SSH"),
        envelope_armor(
            b"BADSIG",
            1,
            [key, b"namespace", b"", b"sha512", &signature],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            2,
            [key, b"namespace", b"", b"sha512", &signature],
            false,
        ),
    ]
}

fn malformed_envelope_payloads(key: &[u8]) -> Vec<String> {
    let signature = signature_blob(b"ssh-ed25519", &[0; 64], false);
    vec![
        envelope_armor(
            b"SSHSIG",
            1,
            [&[0; 51], b"namespace", b"", b"sha512", &signature],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [key, b"namespace", b"", b"sha512", b""],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [
                key,
                b"namespace",
                b"",
                b"sha512",
                &ssh_string_only(b"ssh-ed25519"),
            ],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [key, b"wrong", b"", b"sha512", &signature],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [key, b"namespace", b"x", b"sha512", &signature],
            false,
        ),
    ]
}

fn malformed_signature_payloads(key: &[u8]) -> Vec<String> {
    let signature = signature_blob(b"ssh-ed25519", &[0; 64], false);
    vec![
        envelope_armor(
            b"SSHSIG",
            1,
            [key, b"namespace", b"", b"md5", &signature],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [
                key,
                b"namespace",
                b"",
                b"sha512",
                &signature_blob(b"ssh-rsa", &[0; 64], false),
            ],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [
                key,
                b"namespace",
                b"",
                b"sha512",
                &signature_blob(b"ssh-ed25519", &[0], false),
            ],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [
                key,
                b"namespace",
                b"",
                b"sha512",
                &signature_blob(b"ssh-ed25519", &[0; 64], true),
            ],
            false,
        ),
        envelope_armor(
            b"SSHSIG",
            1,
            [key, b"namespace", b"", b"sha512", &signature],
            true,
        ),
        armor(b"SSHSIG"),
    ]
}

fn ssh_string_only(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    put_string(&mut bytes, value);
    bytes
}
