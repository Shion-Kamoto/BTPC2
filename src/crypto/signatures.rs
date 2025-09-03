use ed25519_dalek::{Signer, Signature, SigningKey, VerifyingKey, SignatureError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureData {
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
}

impl SignatureData {
    pub fn new(signature: Vec<u8>, public_key: Vec<u8>) -> Self {
        SignatureData { signature, public_key }
    }

    pub fn verify(&self, message: &[u8]) -> Result<(), SignatureError> {
        use ed25519_dalek::Verifier;

        // Convert Vec<u8> to fixed-size arrays
        let public_key_bytes: [u8; 32] = self.public_key.clone().try_into()
            .map_err(|_| SignatureError::new())?;
        let signature_bytes: [u8; 64] = self.signature.clone().try_into()
            .map_err(|_| SignatureError::new())?;

        let public_key = VerifyingKey::from_bytes(&public_key_bytes)?;
        let signature = Signature::from_bytes(&signature_bytes);

        public_key.verify(message, &signature)
    }
}

pub type PrivateKey = SigningKey;
pub type PublicKey = VerifyingKey;
pub type KeyPair = (SigningKey, VerifyingKey);

pub fn sha512_hash(data: &[u8]) -> [u8; 64] {
    use sha2::{Sha512, Digest};
    let mut hasher = Sha512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 64];
    hash.copy_from_slice(&result);
    hash
}

pub fn sha512_hash_string(data: &[u8]) -> String {
    hex::encode(sha512_hash(data))
}
