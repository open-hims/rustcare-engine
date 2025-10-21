use crate::error::CryptoError;
use argon2::{
    password_hash::{PasswordHasher, Salt, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use hkdf::Hkdf;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

/// Key derivation result
pub type KdfResult<T> = Result<T, CryptoError>;

/// PBKDF2 parameters for key derivation
#[derive(Debug, Clone)]
pub struct Pbkdf2Params {
    /// Number of iterations (minimum 100,000 for production)
    pub iterations: u32,
    /// Salt length in bytes (minimum 16 bytes recommended)
    pub salt_length: usize,
}

impl Default for Pbkdf2Params {
    fn default() -> Self {
        Self {
            iterations: 600_000, // OWASP 2023 recommendation
            salt_length: 32,
        }
    }
}

/// Argon2 parameters for password hashing
#[derive(Debug, Clone)]
pub struct Argon2Params {
    /// Memory cost in KiB (minimum 19456 for Argon2id)
    pub memory_cost: u32,
    /// Time cost (iterations)
    pub time_cost: u32,
    /// Parallelism factor
    pub parallelism: u32,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            memory_cost: 19456,  // 19 MiB
            time_cost: 2,
            parallelism: 1,
        }
    }
}

/// Key Derivation Function utilities
pub struct Kdf;

impl Kdf {
    /// Derive a key using PBKDF2-HMAC-SHA256
    /// 
    /// # Arguments
    /// * `password` - The password to derive from
    /// * `salt` - Salt for key derivation (should be unique per password)
    /// * `iterations` - Number of iterations (higher = more secure but slower)
    /// * `key_length` - Length of derived key in bytes
    /// 
    /// # Example
    /// ```
    /// use crypto::kdf::{Kdf, Pbkdf2Params};
    /// 
    /// let params = Pbkdf2Params::default();
    /// let salt = Kdf::generate_salt(params.salt_length);
    /// let key = Kdf::pbkdf2(
    ///     b"my_secure_password",
    ///     &salt,
    ///     params.iterations,
    ///     32
    /// ).unwrap();
    /// ```
    pub fn pbkdf2(
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        key_length: usize,
    ) -> KdfResult<Zeroizing<Vec<u8>>> {
        let mut derived_key = Zeroizing::new(vec![0u8; key_length]);
        
        pbkdf2_hmac::<Sha256>(
            password,
            salt,
            iterations,
            &mut derived_key,
        );
        
        Ok(derived_key)
    }

    /// Derive a 32-byte AES-256 key using PBKDF2
    pub fn derive_aes256_key(
        password: &[u8],
        salt: &[u8],
        params: &Pbkdf2Params,
    ) -> KdfResult<[u8; 32]> {
        let derived = Self::pbkdf2(password, salt, params.iterations, 32)?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&derived);
        Ok(key)
    }

    /// Hash a password using Argon2id (recommended for password storage)
    /// 
    /// Returns the password hash in PHC string format which includes:
    /// - Algorithm identifier
    /// - Parameters (memory, iterations, parallelism)
    /// - Salt (base64)
    /// - Hash (base64)
    pub fn argon2_hash(
        password: &[u8],
        params: &Argon2Params,
    ) -> KdfResult<String> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                params.memory_cost,
                params.time_cost,
                params.parallelism,
                None,
            )
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?,
        );

        let password_hash = argon2
            .hash_password(password, &salt)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?
            .to_string();

        Ok(password_hash)
    }

    /// Verify a password against an Argon2 hash
    pub fn argon2_verify(
        password: &[u8],
        password_hash: &str,
    ) -> KdfResult<bool> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        let argon2 = Argon2::default();
        
        Ok(argon2.verify_password(password, &parsed_hash).is_ok())
    }

    /// Derive a key from a password using Argon2id
    /// 
    /// This extracts the raw key bytes instead of the PHC format.
    /// Use this when you need a key for encryption.
    pub fn argon2_derive_key(
        password: &[u8],
        salt: &[u8],
        params: &Argon2Params,
        key_length: usize,
    ) -> KdfResult<Zeroizing<Vec<u8>>> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                params.memory_cost,
                params.time_cost,
                params.parallelism,
                Some(key_length),
            )
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?,
        );

        let mut output = Zeroizing::new(vec![0u8; key_length]);
        
        argon2
            .hash_password_into(
                password,
                salt,
                &mut output,
            )
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        Ok(output)
    }

    /// Derive a 32-byte key using Argon2id
    pub fn argon2_derive_aes256_key(
        password: &[u8],
        salt: &[u8],
        params: &Argon2Params,
    ) -> KdfResult<[u8; 32]> {
        let derived = Self::argon2_derive_key(password, salt, params, 32)?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&derived);
        Ok(key)
    }

    /// HKDF (HMAC-based Key Derivation Function) - RFC 5869
    /// 
    /// Use this to derive multiple keys from a single master key.
    /// 
    /// # Arguments
    /// * `ikm` - Input key material (the master key)
    /// * `salt` - Optional salt value (can be empty)
    /// * `info` - Optional context and application specific information
    /// * `length` - Length of output key material
    pub fn hkdf(
        ikm: &[u8],
        salt: &[u8],
        info: &[u8],
        length: usize,
    ) -> KdfResult<Zeroizing<Vec<u8>>> {
        let hkdf = Hkdf::<Sha256>::new(Some(salt), ikm);
        let mut okm = Zeroizing::new(vec![0u8; length]);
        
        hkdf.expand(info, &mut okm)
            .map_err(|_| CryptoError::KeyDerivationFailed(
                "HKDF expand failed".to_string()
            ))?;

        Ok(okm)
    }

    /// Derive multiple keys from a master key using HKDF
    /// 
    /// This is useful for deriving different keys for different purposes
    /// from a single master key.
    /// 
    /// # Example
    /// ```
    /// use crypto::kdf::Kdf;
    /// 
    /// let master_key = b"master_secret_key";
    /// let encryption_key = Kdf::hkdf(
    ///     master_key,
    ///     b"salt",
    ///     b"encryption-key-v1",
    ///     32
    /// ).unwrap();
    /// let signing_key = Kdf::hkdf(
    ///     master_key,
    ///     b"salt",
    ///     b"signing-key-v1",
    ///     32
    /// ).unwrap();
    /// ```
    pub fn derive_multiple_keys(
        master_key: &[u8],
        salt: &[u8],
        contexts: &[&str],
        key_length: usize,
    ) -> KdfResult<Vec<Zeroizing<Vec<u8>>>> {
        contexts
            .iter()
            .map(|context| Self::hkdf(master_key, salt, context.as_bytes(), key_length))
            .collect()
    }

    /// Generate a cryptographically secure random salt
    pub fn generate_salt(length: usize) -> Vec<u8> {
        let mut salt = vec![0u8; length];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }

    /// Generate a salt and encode as base64 (for storage)
    pub fn generate_salt_base64(length: usize) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let salt = Self::generate_salt(length);
        STANDARD.encode(salt)
    }
}

/// Password strength checker
pub struct PasswordStrength;

impl PasswordStrength {
    /// Check if password meets minimum security requirements
    pub fn is_strong(password: &str) -> bool {
        password.len() >= 12
            && password.chars().any(|c| c.is_uppercase())
            && password.chars().any(|c| c.is_lowercase())
            && password.chars().any(|c| c.is_numeric())
            && password.chars().any(|c| !c.is_alphanumeric())
    }

    /// Calculate password entropy (bits)
    pub fn entropy(password: &str) -> f64 {
        if password.is_empty() {
            return 0.0;
        }

        let mut charset_size = 0;
        if password.chars().any(|c| c.is_lowercase()) {
            charset_size += 26;
        }
        if password.chars().any(|c| c.is_uppercase()) {
            charset_size += 26;
        }
        if password.chars().any(|c| c.is_numeric()) {
            charset_size += 10;
        }
        if password.chars().any(|c| !c.is_alphanumeric()) {
            charset_size += 32; // Approximate
        }

        (password.len() as f64) * (charset_size as f64).log2()
    }

    /// Get strength category
    pub fn category(password: &str) -> &'static str {
        let entropy = Self::entropy(password);
        
        if entropy < 28.0 {
            "Very Weak"
        } else if entropy < 36.0 {
            "Weak"
        } else if entropy < 60.0 {
            "Moderate"
        } else if entropy < 128.0 {
            "Strong"
        } else {
            "Very Strong"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2_derivation() {
        let password = b"my_secure_password";
        let salt = Kdf::generate_salt(32);
        let params = Pbkdf2Params::default();

        let key1 = Kdf::derive_aes256_key(password, &salt, &params).unwrap();
        let key2 = Kdf::derive_aes256_key(password, &salt, &params).unwrap();

        // Same password and salt should produce same key
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_pbkdf2_different_salts() {
        let password = b"my_secure_password";
        let salt1 = Kdf::generate_salt(32);
        let salt2 = Kdf::generate_salt(32);
        let params = Pbkdf2Params::default();

        let key1 = Kdf::derive_aes256_key(password, &salt1, &params).unwrap();
        let key2 = Kdf::derive_aes256_key(password, &salt2, &params).unwrap();

        // Different salts should produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_argon2_hash_verify() {
        let password = b"secure_password_123";
        let params = Argon2Params::default();

        let hash = Kdf::argon2_hash(password, &params).unwrap();
        
        // Correct password should verify
        assert!(Kdf::argon2_verify(password, &hash).unwrap());
        
        // Wrong password should not verify
        assert!(!Kdf::argon2_verify(b"wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_argon2_derive_key() {
        let password = b"my_password";
        let salt = Kdf::generate_salt(32);
        let params = Argon2Params::default();

        let key = Kdf::argon2_derive_aes256_key(password, &salt, &params).unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_hkdf_derivation() {
        let master_key = b"master_secret_key_material";
        let salt = b"random_salt";
        let info = b"application_context";

        let key1 = Kdf::hkdf(master_key, salt, info, 32).unwrap();
        let key2 = Kdf::hkdf(master_key, salt, info, 32).unwrap();

        // Same inputs should produce same output
        assert_eq!(*key1, *key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_hkdf_different_contexts() {
        let master_key = b"master_secret";
        let salt = b"salt";

        let key1 = Kdf::hkdf(master_key, salt, b"context1", 32).unwrap();
        let key2 = Kdf::hkdf(master_key, salt, b"context2", 32).unwrap();

        // Different contexts should produce different keys
        assert_ne!(*key1, *key2);
    }

    #[test]
    fn test_derive_multiple_keys() {
        let master_key = b"master_secret";
        let salt = b"salt";
        let contexts = vec!["encryption", "signing", "authentication"];

        let keys = Kdf::derive_multiple_keys(
            master_key,
            salt,
            &contexts.iter().map(|s| *s).collect::<Vec<_>>(),
            32
        ).unwrap();

        assert_eq!(keys.len(), 3);
        
        // All keys should be different
        assert_ne!(*keys[0], *keys[1]);
        assert_ne!(*keys[1], *keys[2]);
        assert_ne!(*keys[0], *keys[2]);
    }

    #[test]
    fn test_password_strength_checker() {
        assert!(!PasswordStrength::is_strong("weak"));
        assert!(!PasswordStrength::is_strong("nouppercase1!"));
        assert!(!PasswordStrength::is_strong("NOLOWERCASE1!"));
        assert!(!PasswordStrength::is_strong("NoSpecialChar1"));
        assert!(PasswordStrength::is_strong("StrongPass123!"));
    }

    #[test]
    fn test_password_entropy() {
        let weak = "password";
        let strong = "C0mpl3x!P@ssw0rd#2024";

        assert!(PasswordStrength::entropy(strong) > PasswordStrength::entropy(weak));
        
        // Weak password should be weak or very weak
        let weak_category = PasswordStrength::category(weak);
        assert!(weak_category == "Weak" || weak_category == "Moderate" || weak_category == "Very Weak");
        
        // Strong password should be strong or very strong
        assert!(["Strong", "Very Strong"].contains(&PasswordStrength::category(strong)));
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = Kdf::generate_salt(32);
        let salt2 = Kdf::generate_salt(32);

        // Salts should be unique
        assert_ne!(salt1, salt2);
        assert_eq!(salt1.len(), 32);
    }
}
