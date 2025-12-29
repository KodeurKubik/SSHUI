use anyhow::Result;
use russh::keys::PrivateKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SshEd25519 {
    bytes: Vec<u8>,
}

pub fn get_ssh_key() -> Result<PrivateKey> {
    let entry = keyring::Entry::new("sshui", "ssh_ed25519")?;

    let key_pair;
    if let Ok(bytes) = entry.get_password() {
        let deserialized: SshEd25519 = serde_json::from_str(&bytes)?;
        key_pair = PrivateKey::from_bytes(&deserialized.bytes)?;
    } else {
        let mut rng = russh::keys::ssh_key::rand_core::OsRng;
        key_pair = PrivateKey::random(&mut rng, russh::keys::Algorithm::Ed25519)?;

        let serialized = SshEd25519 {
            bytes: key_pair.to_bytes()?.to_vec(),
        };

        let json = serde_json::to_string(&serialized)?;

        entry.set_password(&json)?;
    }

    Ok(key_pair)
}

pub fn get_debug_ssh_key() -> Result<PrivateKey> {
    // a large constant default key - will only be used in non release builds
    let key_pair = PrivateKey::from_bytes(&[
        111, 112, 101, 110, 115, 115, 104, 45, 107, 101, 121, 45, 118, 49, 0, 0, 0, 0, 4, 110, 111,
        110, 101, 0, 0, 0, 4, 110, 111, 110, 101, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 51, 0, 0, 0, 11,
        115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 84, 27, 126, 103, 97, 239,
        55, 231, 171, 215, 60, 204, 55, 206, 162, 184, 249, 15, 71, 215, 245, 215, 225, 75, 130,
        173, 187, 182, 181, 10, 210, 100, 0, 0, 0, 136, 221, 168, 235, 133, 221, 168, 235, 133, 0,
        0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 84, 27, 126, 103,
        97, 239, 55, 231, 171, 215, 60, 204, 55, 206, 162, 184, 249, 15, 71, 215, 245, 215, 225,
        75, 130, 173, 187, 182, 181, 10, 210, 100, 0, 0, 0, 64, 210, 4, 195, 187, 64, 1, 160, 227,
        81, 37, 130, 221, 200, 21, 20, 6, 189, 46, 189, 110, 242, 46, 67, 183, 141, 49, 192, 198,
        153, 195, 61, 43, 84, 27, 126, 103, 97, 239, 55, 231, 171, 215, 60, 204, 55, 206, 162, 184,
        249, 15, 71, 215, 245, 215, 225, 75, 130, 173, 187, 182, 181, 10, 210, 100, 0, 0, 0, 0, 1,
        2, 3, 4, 5,
    ])?;

    Ok(key_pair)
}
