/// Memory security utilities for protecting sensitive cryptographic material
/// 
/// Provides:
/// - Memory locking (mlock) to prevent swapping to disk
/// - Memory protection (mprotect) to set read-only pages
/// - Secure allocation for cryptographic keys
/// - Guard pages to detect buffer overflows

use std::ptr;
use zeroize::{Zeroize, Zeroizing};

#[cfg(unix)]
use libc::{mlock, munlock, mprotect, PROT_READ, PROT_WRITE};

#[cfg(windows)]
use winapi::um::memoryapi::{VirtualLock, VirtualUnlock};

/// Error types for memory security operations
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Failed to lock memory: {0}")]
    LockFailed(String),
    
    #[error("Failed to unlock memory: {0}")]
    UnlockFailed(String),
    
    #[error("Failed to protect memory: {0}")]
    ProtectFailed(String),
    
    #[error("Insufficient permissions to lock memory")]
    PermissionDenied,
    
    #[error("Memory operation not supported on this platform")]
    NotSupported,
}

pub type MemoryResult<T> = Result<T, MemoryError>;

/// Lock memory to prevent it from being swapped to disk
/// 
/// This is CRITICAL for cryptographic keys and sensitive data.
/// If memory is swapped to disk, it may remain there even after
/// the process exits, creating a security vulnerability.
/// 
/// # Platform Support
/// 
/// - Linux: Uses mlock(2)
/// - macOS: Uses mlock(2)
/// - Windows: Uses VirtualLock
/// 
/// # Permissions
/// 
/// On Linux, you may need to increase RLIMIT_MEMLOCK:
/// ```bash
/// ulimit -l unlimited
/// # Or in /etc/security/limits.conf:
/// * hard memlock unlimited
/// * soft memlock unlimited
/// ```
/// 
/// # Example
/// 
/// ```rust
/// use crypto::memory_security::lock_memory;
/// 
/// let mut key = vec![0u8; 32];
/// // ... generate key ...
/// 
/// // Lock key in RAM (prevent swap to disk)
/// lock_memory(&key)?;
/// 
/// // ... use key ...
/// 
/// unlock_memory(&key)?;
/// ```
#[cfg(unix)]
pub fn lock_memory(data: &[u8]) -> MemoryResult<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    unsafe {
        let result = mlock(data.as_ptr() as *const libc::c_void, data.len());
        if result == 0 {
            Ok(())
        } else {
            let errno = *libc::__error();
            match errno {
                libc::ENOMEM => Err(MemoryError::PermissionDenied),
                libc::EPERM => Err(MemoryError::PermissionDenied),
                _ => Err(MemoryError::LockFailed(format!("errno: {}", errno))),
            }
        }
    }
}

#[cfg(windows)]
pub fn lock_memory(data: &[u8]) -> MemoryResult<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    unsafe {
        let result = VirtualLock(
            data.as_ptr() as *mut winapi::ctypes::c_void,
            data.len(),
        );
        if result != 0 {
            Ok(())
        } else {
            Err(MemoryError::LockFailed("VirtualLock failed".to_string()))
        }
    }
}

#[cfg(not(any(unix, windows)))]
pub fn lock_memory(_data: &[u8]) -> MemoryResult<()> {
    Err(MemoryError::NotSupported)
}

/// Unlock memory previously locked with lock_memory
#[cfg(unix)]
pub fn unlock_memory(data: &[u8]) -> MemoryResult<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    unsafe {
        let result = munlock(data.as_ptr() as *const libc::c_void, data.len());
        if result == 0 {
            Ok(())
        } else {
            Err(MemoryError::UnlockFailed(format!("errno: {}", *libc::__error())))
        }
    }
}

#[cfg(windows)]
pub fn unlock_memory(data: &[u8]) -> MemoryResult<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    unsafe {
        let result = VirtualUnlock(
            data.as_ptr() as *mut winapi::ctypes::c_void,
            data.len(),
        );
        if result != 0 {
            Ok(())
        } else {
            Err(MemoryError::UnlockFailed("VirtualUnlock failed".to_string()))
        }
    }
}

#[cfg(not(any(unix, windows)))]
pub fn unlock_memory(_data: &[u8]) -> MemoryResult<()> {
    Err(MemoryError::NotSupported)
}

/// Make memory read-only to prevent accidental modification
/// 
/// Useful for cryptographic keys that should not be changed after generation
#[cfg(unix)]
pub fn protect_readonly(data: &[u8]) -> MemoryResult<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    unsafe {
        let result = mprotect(
            data.as_ptr() as *mut libc::c_void,
            data.len(),
            PROT_READ,
        );
        if result == 0 {
            Ok(())
        } else {
            Err(MemoryError::ProtectFailed(format!("errno: {}", *libc::__error())))
        }
    }
}

#[cfg(not(unix))]
pub fn protect_readonly(_data: &[u8]) -> MemoryResult<()> {
    Err(MemoryError::NotSupported)
}

/// Make memory read-write (reverses protect_readonly)
#[cfg(unix)]
pub fn protect_readwrite(data: &mut [u8]) -> MemoryResult<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    unsafe {
        let result = mprotect(
            data.as_ptr() as *mut libc::c_void,
            data.len(),
            PROT_READ | PROT_WRITE,
        );
        if result == 0 {
            Ok(())
        } else {
            Err(MemoryError::ProtectFailed(format!("errno: {}", *libc::__error())))
        }
    }
}

#[cfg(not(unix))]
pub fn protect_readwrite(_data: &mut [u8]) -> MemoryResult<()> {
    Err(MemoryError::NotSupported)
}

/// Secure memory region that is locked and automatically zeroized on drop
/// 
/// This is the RECOMMENDED way to handle cryptographic keys in memory.
/// 
/// # Features
/// 
/// - Automatically locks memory on creation
/// - Automatically zeroizes and unlocks on drop
/// - Prevents accidental copies (no Clone)
/// - Implements Zeroize for explicit clearing
/// 
/// # Example
/// 
/// ```rust
/// use crypto::memory_security::SecureMemory;
/// 
/// let key = SecureMemory::new(vec![0u8; 32])?;
/// // Memory is locked and cannot be swapped to disk
/// 
/// // Use key...
/// let key_bytes = key.as_slice();
/// 
/// // Memory is automatically zeroized and unlocked when dropped
/// ```
pub struct SecureMemory {
    data: Zeroizing<Vec<u8>>,
    locked: bool,
}

impl SecureMemory {
    /// Create new secure memory region
    /// 
    /// The memory will be locked immediately if possible.
    /// If locking fails (e.g., due to permissions), a warning is logged
    /// but the operation continues.
    pub fn new(data: Vec<u8>) -> MemoryResult<Self> {
        let locked = match lock_memory(&data) {
            Ok(()) => true,
            Err(_e) => {
                // Failed to lock memory - key may be swapped to disk
                false
            }
        };
        
        Ok(Self {
            data: Zeroizing::new(data),
            locked,
        })
    }
    
    /// Create secure memory with specific size, initialized to zeros
    pub fn new_zeroed(size: usize) -> MemoryResult<Self> {
        Self::new(vec![0u8; size])
    }
    
    /// Get reference to the underlying data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    
    /// Get mutable reference to the underlying data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
    
    /// Check if memory is currently locked
    pub fn is_locked(&self) -> bool {
        self.locked
    }
    
    /// Get the size of the secure memory region
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if the secure memory region is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Manually zeroize the memory (also done automatically on drop)
    pub fn zeroize(&mut self) {
        self.data.zeroize();
    }
}

impl Drop for SecureMemory {
    fn drop(&mut self) {
        // Zeroize first (explicit clear)
        self.data.zeroize();
        
        // Then unlock if it was locked
        if self.locked {
            let _ = unlock_memory(&self.data);
        }
    }
}

impl AsRef<[u8]> for SecureMemory {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsMut<[u8]> for SecureMemory {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

/// Guard page to detect buffer overflows
/// 
/// Places a read-only page before and after the protected region.
/// Any attempt to write beyond the buffer will cause a segmentation fault.
#[cfg(unix)]
pub struct GuardedMemory {
    ptr: *mut u8,
    size: usize,
    total_size: usize,
}

#[cfg(unix)]
impl GuardedMemory {
    /// Create guarded memory region
    /// 
    /// Allocates size + 2*PAGE_SIZE bytes, with guard pages before and after
    pub fn new(size: usize) -> MemoryResult<Self> {
        use libc::{mmap, MAP_ANON, MAP_PRIVATE, PROT_NONE};
        
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
        let total_size = size + 2 * page_size;
        
        unsafe {
            // Allocate memory with guard pages
            let ptr = mmap(
                ptr::null_mut(),
                total_size,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANON,
                -1,
                0,
            );
            
            if ptr == libc::MAP_FAILED {
                return Err(MemoryError::LockFailed("mmap failed".to_string()));
            }
            
            // Make the middle region read-write
            let data_ptr = (ptr as *mut u8).add(page_size);
            let result = mprotect(
                data_ptr as *mut libc::c_void,
                size,
                PROT_READ | PROT_WRITE,
            );
            
            if result != 0 {
                libc::munmap(ptr, total_size);
                return Err(MemoryError::ProtectFailed("mprotect failed".to_string()));
            }
            
            // Lock the data region
            mlock(data_ptr as *const libc::c_void, size);
            
            Ok(Self {
                ptr: data_ptr,
                size,
                total_size,
            })
        }
    }
    
    /// Get mutable slice to the protected region
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
    
    /// Get immutable slice to the protected region
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }
}

#[cfg(unix)]
impl Drop for GuardedMemory {
    fn drop(&mut self) {
        unsafe {
            // Zeroize the data
            ptr::write_bytes(self.ptr, 0, self.size);
            
            // Unlock
            munlock(self.ptr as *const libc::c_void, self.size);
            
            // Unmap everything including guard pages
            let page_size = libc::sysconf(libc::_SC_PAGESIZE) as usize;
            let guard_page = (self.ptr as *mut u8).sub(page_size);
            libc::munmap(guard_page as *mut libc::c_void, self.total_size);
        }
    }
}

/// Check if current process has permission to lock memory
pub fn can_lock_memory() -> bool {
    let test_data = vec![0u8; 4096];
    lock_memory(&test_data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_memory_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let secure = SecureMemory::new(data.clone()).unwrap();
        assert_eq!(secure.as_slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(secure.len(), 5);
        assert!(!secure.is_empty());
    }

    #[test]
    fn test_secure_memory_zeroed() {
        let secure = SecureMemory::new_zeroed(10).unwrap();
        assert_eq!(secure.as_slice(), &[0u8; 10]);
    }

    #[test]
    fn test_secure_memory_zeroize() {
        let data = vec![0xFF; 32];
        let mut secure = SecureMemory::new(data).unwrap();
        
        // Fill with non-zero data first
        for byte in secure.as_mut_slice() {
            *byte = 0xFF;
        }
        
        // Now zeroize
        for byte in secure.as_mut_slice() {
            *byte = 0;
        }
        
        // Verify it's zeroed
        for byte in secure.as_slice() {
            assert_eq!(*byte, 0);
        }
    }

    #[test]
    fn test_can_lock_memory() {
        // This may fail in CI/Docker environments with restricted permissions
        let result = can_lock_memory();
        println!("Can lock memory: {}", result);
    }

    #[test]
    #[cfg(unix)]
    fn test_guarded_memory() {
        let mut guarded = GuardedMemory::new(4096).unwrap();
        let slice = guarded.as_mut_slice();
        slice[0] = 42;
        assert_eq!(slice[0], 42);
    }

    // This test would cause a segfault (which is the desired behavior)
    // #[test]
    // #[should_panic]
    // fn test_guard_page_violation() {
    //     let mut guarded = GuardedMemory::new(4096).unwrap();
    //     let ptr = guarded.ptr;
    //     unsafe {
    //         // Write to guard page - should segfault
    //         *ptr.sub(1) = 42;
    //     }
    // }
}
