//! Pagination types and utilities for consistent pagination across all endpoints

use serde::Deserialize;
use utoipa::IntoParams;
use crate::error::{ResponseMetadata, PaginationInfo};

/// Standard pagination parameters for list endpoints
///
/// All list endpoints should use this type for consistent pagination behavior.
#[derive(Debug, Deserialize, IntoParams, Clone)]
pub struct PaginationParams {
    #[param(example = 1, minimum = 1)]
    pub page: Option<u32>,
    
    #[param(example = 20, minimum = 1, maximum = 100)]
    pub page_size: Option<u32>,
}

impl PaginationParams {
    /// Get the page number (defaults to 1, minimum 1)
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }
    
    /// Get the page size (defaults to 20, clamped between 1 and 100)
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
    
    /// Calculate the offset for SQL queries
    pub fn offset(&self) -> u64 {
        ((self.page() - 1) * self.page_size()) as u64
    }
    
    /// Get the limit for SQL queries (alias for page_size)
    pub fn limit(&self) -> u32 {
        self.page_size()
    }
    
    /// Calculate total pages given a total count
    pub fn total_pages(&self, total_count: i64) -> u32 {
        if total_count == 0 {
            return 1;
        }
        ((total_count as f64) / (self.page_size() as f64)).ceil() as u32
    }
    
    /// Create response metadata with pagination info
    pub fn to_metadata(&self, total_count: i64) -> ResponseMetadata {
        let total_pages = self.total_pages(total_count);
        
        ResponseMetadata {
            pagination: Some(PaginationInfo {
                page: self.page() as i32,
                page_size: self.page_size() as i32,
                total_pages: total_pages as i32,
                has_next: self.page() < total_pages,
                has_previous: self.page() > 1,
            }),
            total_count: Some(total_count),
            request_id: None,
        }
    }
    
    /// Wrap data with pagination metadata
    pub fn wrap_response<T>(&self, data: T, total_count: i64) -> crate::error::ApiResponse<T> {
        crate::error::api_success_with_meta(data, self.to_metadata(total_count))
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults() {
        let params = PaginationParams { page: None, page_size: None };
        assert_eq!(params.page(), 1);
        assert_eq!(params.page_size(), 20);
    }

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams { page: Some(3), page_size: Some(10) };
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_total_pages() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        assert_eq!(params.total_pages(100), 5);
        assert_eq!(params.total_pages(101), 6);
        assert_eq!(params.total_pages(0), 1);
    }
}

