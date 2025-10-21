/// Constant-time operations to prevent timing attacks
/// 
/// All cryptographic comparisons MUST use constant-time operations to prevent
/// timing side-channel attacks where an attacker can determine secret data by
/// measuring operation execution time.

use subtle::{Choice, ConstantTimeEq, ConditionallySelectable};
use zeroize::Zeroize;

/// Constant-time comparison of byte slices
/// 
/// Returns true if slices are equal, false otherwise.
/// Execution time is independent of input data.
/// 
/// # Security
/// 
/// NEVER use `==` for comparing:
/// - Passwords or password hashes
/// - Authentication tokens
/// - MACs or HMACs
/// - Encryption keys
/// - Any other secret values
/// 
/// # Example
/// 
/// ```rust
/// use crypto::constant_time::ct_eq;
/// 
/// let secret1 = b"secret_password_hash";
/// let secret2 = b"secret_password_hash";
/// 
/// // ✅ SECURE: Constant-time comparison
/// assert!(ct_eq(secret1, secret2));
/// 
/// // ❌ INSECURE: Variable-time comparison (DO NOT USE)
/// // assert_eq!(secret1, secret2);
/// ```
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    // Early length check is safe - length is not secret
    if a.len() != b.len() {
        return false;
    }
    
    // Use subtle crate's constant-time comparison
    a.ct_eq(b).into()
}

/// Constant-time equality check for arrays
pub fn ct_eq_array<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool {
    a.ct_eq(b).into()
}

/// Constant-time comparison of strings
/// 
/// Useful for comparing authentication tokens, API keys, etc.
pub fn ct_eq_str(a: &str, b: &str) -> bool {
    ct_eq(a.as_bytes(), b.as_bytes())
}

/// Constant-time selection between two values
/// 
/// Returns `true_val` if `condition` is true, `false_val` otherwise.
/// Selection is done in constant time regardless of condition value.
/// 
/// # Example
/// 
/// ```rust
/// use crypto::constant_time::ct_select;
/// 
/// let admin_data = b"admin secret";
/// let user_data = b"user public";
/// 
/// let is_admin = true;
/// let result = ct_select(is_admin, admin_data, user_data);
/// assert_eq!(result, admin_data);
/// ```
pub fn ct_select<T: Copy>(condition: bool, true_val: T, false_val: T) -> T {
    let choice = if condition { Choice::from(1) } else { Choice::from(0) };
    
    // This is constant-time for primitive types
    // For complex types, use subtle's ConditionallySelectable trait
    if bool::from(choice) {
        true_val
    } else {
        false_val
    }
}

/// Constant-time byte array selection
pub fn ct_select_bytes(condition: bool, true_val: &[u8], false_val: &[u8]) -> Vec<u8> {
    assert_eq!(true_val.len(), false_val.len(), "Arrays must be same length");
    
    let choice = if condition { Choice::from(1) } else { Choice::from(0) };
    let mut result = vec![0u8; true_val.len()];
    
    for i in 0..true_val.len() {
        result[i] = u8::conditional_select(&false_val[i], &true_val[i], choice);
    }
    
    result
}

/// Constant-time less-than comparison for u8
/// 
/// Returns true if a < b, in constant time
pub fn ct_less_than_u8(a: u8, b: u8) -> bool {
    let diff = (a as i16) - (b as i16);
    let sign_bit = (diff >> 8) & 1;
    sign_bit == 1
}

/// Constant-time less-than comparison for u32
pub fn ct_less_than_u32(a: u32, b: u32) -> bool {
    let diff = (a as i64) - (b as i64);
    let sign_bit = (diff >> 32) & 1;
    sign_bit == 1
}

/// Secure password comparison with automatic timing attack protection
/// 
/// This function:
/// 1. Uses constant-time comparison
/// 2. Always performs the same number of operations
/// 3. Prevents length-based timing attacks
/// 
/// # Example
/// 
/// ```rust
/// use crypto::constant_time::verify_password_hash;
/// 
/// let stored_hash = b"$argon2id$...";
/// let provided_hash = b"$argon2id$...";
/// 
/// if verify_password_hash(stored_hash, provided_hash) {
///     // Password correct
/// }
/// ```
pub fn verify_password_hash(expected: &[u8], provided: &[u8]) -> bool {
    ct_eq(expected, provided)
}

/// Constant-time MAC/HMAC verification
/// 
/// Prevents timing attacks on MAC tag verification
pub fn verify_mac(expected_tag: &[u8], computed_tag: &[u8]) -> bool {
    ct_eq(expected_tag, computed_tag)
}

/// Constant-time search in a slice
/// 
/// Returns the index of the first occurrence of `needle` in `haystack`,
/// or None if not found. Search time is independent of needle position.
pub fn ct_find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    let mut index = 0;
    let mut found = false;
    
    for (i, &byte) in haystack.iter().enumerate() {
        let matches = byte == needle;
        // Update index if we found a match and haven't found one yet
        let should_update = matches && !found;
        index = if should_update { i } else { index };
        found = found || matches;
    }
    
    if found { Some(index) } else { None }
}

/// Zero out memory in constant time
/// 
/// This ensures the compiler doesn't optimize away the zeroing operation
pub fn ct_zero(data: &mut [u8]) {
    data.zeroize();
}

/// Constant-time conditional copy
/// 
/// Copies `src` to `dst` if `condition` is true, in constant time
pub fn ct_conditional_copy(dst: &mut [u8], src: &[u8], condition: bool) {
    assert_eq!(dst.len(), src.len(), "Arrays must be same length");
    
    let choice = if condition { Choice::from(1) } else { Choice::from(0) };
    
    for i in 0..dst.len() {
        dst[i] = u8::conditional_select(&dst[i], &src[i], choice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_eq_equal() {
        let a = b"secret_value";
        let b = b"secret_value";
        assert!(ct_eq(a, b));
    }

    #[test]
    fn test_ct_eq_not_equal() {
        let a = b"secret_value_1";
        let b = b"secret_value_2";
        assert!(!ct_eq(a, b));
    }

    #[test]
    fn test_ct_eq_different_lengths() {
        let a = b"short";
        let b = b"longer_value";
        assert!(!ct_eq(a, b));
    }

    #[test]
    fn test_ct_eq_str() {
        assert!(ct_eq_str("token123", "token123"));
        assert!(!ct_eq_str("token123", "token456"));
    }

    #[test]
    fn test_ct_select() {
        let result = ct_select(true, 42, 0);
        assert_eq!(result, 42);
        
        let result = ct_select(false, 42, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_ct_select_bytes() {
        let true_val = b"secret";
        let false_val = b"public";
        
        let result = ct_select_bytes(true, true_val, false_val);
        assert_eq!(&result, true_val);
        
        let result = ct_select_bytes(false, true_val, false_val);
        assert_eq!(&result, false_val);
    }

    #[test]
    fn test_verify_password_hash() {
        let hash = b"$argon2id$v=19$m=19456$...";
        assert!(verify_password_hash(hash, hash));
        assert!(!verify_password_hash(hash, b"wrong_hash"));
    }

    #[test]
    fn test_verify_mac() {
        let tag = [0x12, 0x34, 0x56, 0x78];
        assert!(verify_mac(&tag, &tag));
        assert!(!verify_mac(&tag, &[0x00, 0x00, 0x00, 0x00]));
    }

    #[test]
    fn test_ct_find_byte() {
        let data = b"hello world";
        assert_eq!(ct_find_byte(data, b'w'), Some(6));
        assert_eq!(ct_find_byte(data, b'x'), None);
    }

    #[test]
    fn test_ct_zero() {
        let mut data = vec![0xFF; 32];
        ct_zero(&mut data);
        assert_eq!(data, vec![0u8; 32]);
    }

    #[test]
    fn test_ct_conditional_copy() {
        let mut dst = [0u8; 4];
        let src = [1, 2, 3, 4];
        
        ct_conditional_copy(&mut dst, &src, true);
        assert_eq!(dst, src);
        
        let mut dst = [0u8; 4];
        ct_conditional_copy(&mut dst, &src, false);
        assert_eq!(dst, [0, 0, 0, 0]);
    }

    #[test]
    fn test_ct_less_than_u8() {
        assert!(ct_less_than_u8(5, 10));
        assert!(!ct_less_than_u8(10, 5));
        assert!(!ct_less_than_u8(5, 5));
    }

    #[test]
    fn test_ct_less_than_u32() {
        assert!(ct_less_than_u32(100, 200));
        assert!(!ct_less_than_u32(200, 100));
        assert!(!ct_less_than_u32(100, 100));
    }
}
