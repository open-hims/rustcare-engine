use crate::aes_gcm::Aes256GcmEncryptor;
use crate::encryption::Encryptor;
use crate::error::CryptoError;
use crate::kdf::Kdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

/// Result type for envelope encryption operations
pub type EnvelopeResult<T> = Result<T, CryptoError>;

/// Envelope encryption metadata
/// 
/// Contains the encrypted Data Encryption Key (DEK) and parameters needed for decryption.
/// The DEK is encrypted with a Key Encryption Key (KEK).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeMetadata {
    /// Version of envelope encryption format
    pub version: u32,
    /// Encrypted DEK (Data Encryption Key)
    pub encrypted_dek: String,
    /// Algorithm used for DEK encryption
    pub dek_algorithm: String,
    /// Algorithm used for data encryption
    pub data_algorithm: String,
    /// Number of chunks (for chunked encryption)
    pub chunk_count: Option<usize>,
    /// Chunk size in bytes
    pub chunk_size: Option<usize>,
}

/// Envelope encryption for large data
/// 
/// This implements the envelope encryption pattern:
/// 1. Generate a random Data Encryption Key (DEK)
/// 2. Encrypt data with DEK
/// 3. Encrypt DEK with Key Encryption Key (KEK)
/// 4. Store encrypted DEK alongside encrypted data
/// 
/// Benefits:
/// - Only need to manage one KEK (can be stored in KMS)
/// - Can re-encrypt DEK without re-encrypting all data
/// - Efficient for large files
/// - Supports key rotation
pub struct EnvelopeEncryption {
    /// Key Encryption Key (master key)
    kek_encryptor: Aes256GcmEncryptor,
}

impl EnvelopeEncryption {
    /// Create new envelope encryption with a KEK
    pub fn new(kek: [u8; 32]) -> EnvelopeResult<Self> {
        let kek_encryptor = Aes256GcmEncryptor::new(kek)?;
        Ok(Self { kek_encryptor })
    }

    /// Create from base64-encoded KEK
    pub fn from_base64_kek(kek_b64: &str) -> EnvelopeResult<Self> {
        let kek_encryptor = Aes256GcmEncryptor::from_base64(kek_b64)?;
        Ok(Self { kek_encryptor })
    }

    /// Encrypt data using envelope encryption
    /// 
    /// Returns: (encrypted_data, metadata)
    pub fn encrypt(&self, plaintext: &[u8]) -> EnvelopeResult<(Vec<u8>, EnvelopeMetadata)> {
        // 1. Generate random DEK
        let dek = Aes256GcmEncryptor::generate_key();
        let dek_encryptor = Aes256GcmEncryptor::new(dek)?;

        // 2. Encrypt data with DEK
        let encrypted_data = dek_encryptor.encrypt(plaintext)?;

        // 3. Encrypt DEK with KEK
        let encrypted_dek = self.kek_encryptor.encrypt_string(&base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            dek,
        ))?;

        // 4. Create metadata
        let metadata = EnvelopeMetadata {
            version: 1,
            encrypted_dek,
            dek_algorithm: "AES-256-GCM".to_string(),
            data_algorithm: "AES-256-GCM".to_string(),
            chunk_count: None,
            chunk_size: None,
        };

        Ok((encrypted_data, metadata))
    }

    /// Decrypt data using envelope encryption
    pub fn decrypt(
        &self,
        encrypted_data: &[u8],
        metadata: &EnvelopeMetadata,
    ) -> EnvelopeResult<Vec<u8>> {
        // 1. Decrypt DEK with KEK
        let dek_b64 = self.kek_encryptor.decrypt_string(&metadata.encrypted_dek)?;
        let dek_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            dek_b64.as_bytes(),
        )
        .map_err(|e| CryptoError::InvalidFormat(format!("Base64 decode error: {}", e)))?;

        if dek_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: dek_bytes.len(),
            });
        }

        let mut dek = [0u8; 32];
        dek.copy_from_slice(&dek_bytes);

        // 2. Decrypt data with DEK
        let dek_encryptor = Aes256GcmEncryptor::new(dek)?;
        dek_encryptor.decrypt(encrypted_data)
    }

    /// Encrypt large data in chunks
    /// 
    /// This is more memory-efficient for large files as it processes data in chunks.
    /// Each chunk is encrypted with the same DEK.
    pub fn encrypt_chunked(
        &self,
        plaintext: &[u8],
        chunk_size: usize,
    ) -> EnvelopeResult<(Vec<Vec<u8>>, EnvelopeMetadata)> {
        // 1. Generate random DEK
        let dek = Aes256GcmEncryptor::generate_key();
        let dek_encryptor = Aes256GcmEncryptor::new(dek)?;

        // 2. Encrypt data in chunks
        let mut encrypted_chunks = Vec::new();
        for chunk in plaintext.chunks(chunk_size) {
            let encrypted_chunk = dek_encryptor.encrypt(chunk)?;
            encrypted_chunks.push(encrypted_chunk);
        }

        // 3. Encrypt DEK with KEK
        let encrypted_dek = self.kek_encryptor.encrypt_string(&base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            dek,
        ))?;

        // 4. Create metadata
        let metadata = EnvelopeMetadata {
            version: 1,
            encrypted_dek,
            dek_algorithm: "AES-256-GCM".to_string(),
            data_algorithm: "AES-256-GCM".to_string(),
            chunk_count: Some(encrypted_chunks.len()),
            chunk_size: Some(chunk_size),
        };

        Ok((encrypted_chunks, metadata))
    }

    /// Decrypt chunked data
    pub fn decrypt_chunked(
        &self,
        encrypted_chunks: &[Vec<u8>],
        metadata: &EnvelopeMetadata,
    ) -> EnvelopeResult<Vec<u8>> {
        // 1. Decrypt DEK with KEK
        let dek_b64 = self.kek_encryptor.decrypt_string(&metadata.encrypted_dek)?;
        let dek_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            dek_b64.as_bytes(),
        )
        .map_err(|e| CryptoError::InvalidFormat(format!("Base64 decode error: {}", e)))?;

        if dek_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: dek_bytes.len(),
            });
        }

        let mut dek = [0u8; 32];
        dek.copy_from_slice(&dek_bytes);

        // 2. Decrypt each chunk with DEK
        let dek_encryptor = Aes256GcmEncryptor::new(dek)?;
        let mut plaintext = Vec::new();

        for chunk in encrypted_chunks {
            let decrypted_chunk = dek_encryptor.decrypt(chunk)?;
            plaintext.extend_from_slice(&decrypted_chunk);
        }

        Ok(plaintext)
    }

    /// Re-wrap a DEK with a new KEK (for key rotation)
    /// 
    /// This allows changing the KEK without re-encrypting all data.
    pub fn rewrap_dek(
        old_kek: &Aes256GcmEncryptor,
        new_kek: &Aes256GcmEncryptor,
        old_metadata: &EnvelopeMetadata,
    ) -> EnvelopeResult<EnvelopeMetadata> {
        // 1. Decrypt DEK with old KEK
        let dek_b64 = old_kek.decrypt_string(&old_metadata.encrypted_dek)?;

        // 2. Re-encrypt DEK with new KEK
        let new_encrypted_dek = new_kek.encrypt_string(&dek_b64)?;

        // 3. Create new metadata
        let mut new_metadata = old_metadata.clone();
        new_metadata.encrypted_dek = new_encrypted_dek;

        Ok(new_metadata)
    }
}

/// Streaming envelope encryption for very large files
/// 
/// This is useful when the entire file doesn't fit in memory.
pub struct StreamingEnvelopeEncryption {
    envelope: EnvelopeEncryption,
    chunk_size: usize,
}

impl StreamingEnvelopeEncryption {
    /// Create new streaming encryption
    /// 
    /// # Arguments
    /// * `kek` - Key Encryption Key
    /// * `chunk_size` - Size of each chunk in bytes (e.g., 1MB = 1_048_576)
    pub fn new(kek: [u8; 32], chunk_size: usize) -> EnvelopeResult<Self> {
        let envelope = EnvelopeEncryption::new(kek)?;
        Ok(Self {
            envelope,
            chunk_size,
        })
    }

    /// Get the chunk size
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Encrypt a chunk (call this repeatedly for each chunk of the file)
    pub fn encrypt_chunk(&self, chunk: &[u8], dek: &[u8; 32]) -> EnvelopeResult<Vec<u8>> {
        let dek_encryptor = Aes256GcmEncryptor::new(*dek)?;
        dek_encryptor.encrypt(chunk)
    }

    /// Generate and wrap a new DEK
    pub fn generate_wrapped_dek(&self) -> EnvelopeResult<String> {
        let dek = Aes256GcmEncryptor::generate_key();
        let dek_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, dek);
        self.envelope.kek_encryptor.encrypt_string(&dek_b64)
    }

    /// Unwrap a DEK for decryption
    pub fn unwrap_dek(&self, encrypted_dek: &str) -> EnvelopeResult<[u8; 32]> {
        let dek_b64 = self.envelope.kek_encryptor.decrypt_string(encrypted_dek)?;
        let dek_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            dek_b64.as_bytes(),
        )
        .map_err(|e| CryptoError::InvalidFormat(format!("Base64 decode error: {}", e)))?;

        if dek_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: dek_bytes.len(),
            });
        }

        let mut dek = [0u8; 32];
        dek.copy_from_slice(&dek_bytes);
        Ok(dek)
    }

    /// Decrypt a chunk
    pub fn decrypt_chunk(&self, encrypted_chunk: &[u8], dek: &[u8; 32]) -> EnvelopeResult<Vec<u8>> {
        let dek_encryptor = Aes256GcmEncryptor::new(*dek)?;
        dek_encryptor.decrypt(encrypted_chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_encryption_basic() {
        let kek = Aes256GcmEncryptor::generate_key();
        let envelope = EnvelopeEncryption::new(kek).unwrap();

        let plaintext = b"Sensitive healthcare data that needs envelope encryption";
        let (encrypted, metadata) = envelope.encrypt(plaintext).unwrap();

        let decrypted = envelope.decrypt(&encrypted, &metadata).unwrap();
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_envelope_metadata_serialization() {
        let kek = Aes256GcmEncryptor::generate_key();
        let envelope = EnvelopeEncryption::new(kek).unwrap();

        let plaintext = b"test data";
        let (encrypted, metadata) = envelope.encrypt(plaintext).unwrap();

        // Serialize and deserialize metadata
        let metadata_json = serde_json::to_string(&metadata).unwrap();
        let metadata_restored: EnvelopeMetadata = serde_json::from_str(&metadata_json).unwrap();

        // Should still be able to decrypt
        let decrypted = envelope.decrypt(&encrypted, &metadata_restored).unwrap();
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_chunked_encryption() {
        let kek = Aes256GcmEncryptor::generate_key();
        let envelope = EnvelopeEncryption::new(kek).unwrap();

        // Create data larger than typical chunk size
        let plaintext = vec![0x42u8; 10_000];
        let chunk_size = 1024;

        let (encrypted_chunks, metadata) = envelope.encrypt_chunked(&plaintext, chunk_size).unwrap();

        assert_eq!(metadata.chunk_count, Some((plaintext.len() + chunk_size - 1) / chunk_size));
        assert_eq!(metadata.chunk_size, Some(chunk_size));

        let decrypted = envelope.decrypt_chunked(&encrypted_chunks, &metadata).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_large_file_chunked() {
        let kek = Aes256GcmEncryptor::generate_key();
        let envelope = EnvelopeEncryption::new(kek).unwrap();

        // Simulate a 5MB file
        let plaintext = vec![0x55u8; 5 * 1024 * 1024];
        let chunk_size = 1024 * 1024; // 1MB chunks

        let (encrypted_chunks, metadata) = envelope.encrypt_chunked(&plaintext, chunk_size).unwrap();
        let decrypted = envelope.decrypt_chunked(&encrypted_chunks, &metadata).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_eq!(encrypted_chunks.len(), 5);
    }

    #[test]
    fn test_dek_rewrap() {
        let old_kek = Aes256GcmEncryptor::generate_key();
        let new_kek = Aes256GcmEncryptor::generate_key();

        let old_envelope = EnvelopeEncryption::new(old_kek).unwrap();
        let new_envelope = EnvelopeEncryption::new(new_kek).unwrap();

        let plaintext = b"data encrypted with old KEK";
        let (encrypted, old_metadata) = old_envelope.encrypt(plaintext).unwrap();

        // Rewrap DEK
        let old_encryptor = Aes256GcmEncryptor::new(old_kek).unwrap();
        let new_encryptor = Aes256GcmEncryptor::new(new_kek).unwrap();
        let new_metadata = EnvelopeEncryption::rewrap_dek(
            &old_encryptor,
            &new_encryptor,
            &old_metadata,
        )
        .unwrap();

        // Should be able to decrypt with new KEK and new metadata
        let decrypted = new_envelope.decrypt(&encrypted, &new_metadata).unwrap();
        assert_eq!(plaintext, decrypted.as_slice());

        // Old KEK should no longer work
        assert!(old_envelope.decrypt(&encrypted, &new_metadata).is_err());
    }

    #[test]
    fn test_streaming_encryption() {
        let kek = Aes256GcmEncryptor::generate_key();
        let streaming = StreamingEnvelopeEncryption::new(kek, 1024).unwrap();

        // Generate DEK
        let encrypted_dek = streaming.generate_wrapped_dek().unwrap();
        let dek = streaming.unwrap_dek(&encrypted_dek).unwrap();

        // Encrypt chunks
        let chunk1 = b"First chunk of data";
        let chunk2 = b"Second chunk of data";

        let encrypted1 = streaming.encrypt_chunk(chunk1, &dek).unwrap();
        let encrypted2 = streaming.encrypt_chunk(chunk2, &dek).unwrap();

        // Decrypt chunks
        let decrypted1 = streaming.decrypt_chunk(&encrypted1, &dek).unwrap();
        let decrypted2 = streaming.decrypt_chunk(&encrypted2, &dek).unwrap();

        assert_eq!(chunk1, decrypted1.as_slice());
        assert_eq!(chunk2, decrypted2.as_slice());
    }

    #[test]
    fn test_different_deks() {
        let kek = Aes256GcmEncryptor::generate_key();
        let envelope = EnvelopeEncryption::new(kek).unwrap();

        let plaintext = b"same plaintext";
        let (encrypted1, metadata1) = envelope.encrypt(plaintext).unwrap();
        let (encrypted2, metadata2) = envelope.encrypt(plaintext).unwrap();

        // Different DEKs should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);
        assert_ne!(metadata1.encrypted_dek, metadata2.encrypted_dek);

        // But both should decrypt correctly
        assert_eq!(
            envelope.decrypt(&encrypted1, &metadata1).unwrap(),
            plaintext
        );
        assert_eq!(
            envelope.decrypt(&encrypted2, &metadata2).unwrap(),
            plaintext
        );
    }

    #[test]
    fn test_wrong_kek() {
        let kek1 = Aes256GcmEncryptor::generate_key();
        let kek2 = Aes256GcmEncryptor::generate_key();

        let envelope1 = EnvelopeEncryption::new(kek1).unwrap();
        let envelope2 = EnvelopeEncryption::new(kek2).unwrap();

        let plaintext = b"encrypted with kek1";
        let (encrypted, metadata) = envelope1.encrypt(plaintext).unwrap();

        // Decryption with wrong KEK should fail
        assert!(envelope2.decrypt(&encrypted, &metadata).is_err());
    }
}
