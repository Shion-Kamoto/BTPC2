use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Consensus error: {0}")]
    ConsensusError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Wallet error: {0}")]
    WalletError(String),

    #[error("UTXO error: {0}")]
    UtxoError(String),
}

// Implement From for UTXOError to BlockchainError
impl From<crate::database::utxo_set::UTXOError> for BlockchainError {
    fn from(error: crate::database::utxo_set::UTXOError) -> Self {
        BlockchainError::UtxoError(format!("{}", error))
    }
}

// Implement From for ProtocolError to BlockchainError
impl From<crate::network::ProtocolError> for BlockchainError {
    fn from(error: crate::network::ProtocolError) -> Self {
        BlockchainError::NetworkError(format!("{}", error))
    }
}
