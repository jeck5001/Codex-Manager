use sha2::{Digest, Sha256};

pub(crate) fn fingerprint_anchor(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    )
}
