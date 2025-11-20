//! Pagination types and utilities for consistent pagination across all endpoints

use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use crate::error::{ResponseMetadata, PaginationInfo};

/// Standard pagination parameters for list endpoints
///
/// All list endpoints should use this type for consistent pagination behavior.
#[derive(Debug, Deserialize, IntoParams, ToSchema, Clone)]
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
    fn test_pagination_with_values() {
        let params = PaginationParams { page: Some(2), page_size: Some(50) };
        assert_eq!(params.page(), 2);
        assert_eq!(params.page_size(), 50);
    }

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams { page: Some(3), page_size: Some(10) };
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_offset_first_page() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_limit() {
        let params = PaginationParams { page: Some(2), page_size: Some(25) };
        assert_eq!(params.limit(), 25);
        assert_eq!(params.limit(), params.page_size());
    }

    #[test]
    fn test_total_pages() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        assert_eq!(params.total_pages(100), 5);
        assert_eq!(params.total_pages(101), 6);
        assert_eq!(params.total_pages(0), 1);
    }

    #[test]
    fn test_total_pages_exact_divisor() {
        let params = PaginationParams { page: Some(1), page_size: Some(10) };
        assert_eq!(params.total_pages(100), 10);
    }

    #[test]
    fn test_total_pages_less_than_page_size() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        assert_eq!(params.total_pages(15), 1);
    }

    #[test]
    fn test_total_pages_one_more() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        assert_eq!(params.total_pages(21), 2);
    }

    #[test]
    fn test_to_metadata() {
        let params = PaginationParams { page: Some(2), page_size: Some(20) };
        let metadata = params.to_metadata(100);
        
        assert!(metadata.pagination.is_some());
        let pagination = metadata.pagination.unwrap();
        assert_eq!(pagination.page, 2);
        assert_eq!(pagination.page_size, 20);
        assert_eq!(pagination.total_pages, 5);
        assert!(pagination.has_next);
        assert!(pagination.has_previous);
        assert_eq!(metadata.total_count, Some(100));
    }

    #[test]
    fn test_to_metadata_first_page() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        let metadata = params.to_metadata(100);
        
        let pagination = metadata.pagination.unwrap();
        assert!(!pagination.has_previous);
        assert!(pagination.has_next);
    }

    #[test]
    fn test_to_metadata_last_page() {
        let params = PaginationParams { page: Some(5), page_size: Some(20) };
        let metadata = params.to_metadata(100);
        
        let pagination = metadata.pagination.unwrap();
        assert!(pagination.has_previous);
        assert!(!pagination.has_next);
    }

    #[test]
    fn test_to_metadata_single_page() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        let metadata = params.to_metadata(15);
        
        let pagination = metadata.pagination.unwrap();
        assert_eq!(pagination.total_pages, 1);
        assert!(!pagination.has_previous);
        assert!(!pagination.has_next);
    }

    #[test]
    fn test_to_metadata_empty_results() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        let metadata = params.to_metadata(0);
        
        let pagination = metadata.pagination.unwrap();
        assert_eq!(pagination.total_pages, 1);
        assert_eq!(metadata.total_count, Some(0));
    }

    #[test]
    fn test_wrap_response() {
        let params = PaginationParams { page: Some(1), page_size: Some(20) };
        let data = vec!["item1", "item2"];
        let response = params.wrap_response(data.clone(), 2);
        
        assert!(response.metadata.is_some());
        let metadata = response.metadata.unwrap();
        assert_eq!(metadata.total_count, Some(2));
    }

    #[test]
    fn test_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, Some(1));
        assert_eq!(params.page_size, Some(20));
    }

    #[test]
    fn test_page_min_clamp() {
        let params = PaginationParams { page: Some(0), page_size: Some(20) };
        assert_eq!(params.page(), 1); // Should clamp to 1
    }

    #[test]
    fn test_page_size_max_clamp() {
        let params = PaginationParams { page: Some(1), page_size: Some(200) };
        assert_eq!(params.page_size(), 100); // Should clamp to 100
    }

    #[test]
    fn test_page_size_min_clamp() {
        let params = PaginationParams { page: Some(1), page_size: Some(0) };
        assert_eq!(params.page_size(), 1); // Should clamp to 1
    }
}

