//! Helper macros for common patterns in RustCare server
//!
//! This module provides macros to reduce boilerplate, particularly for
//! OpenAPI documentation and common code patterns.

/// Generate utoipa::path attribute for a list endpoint
///
/// This macro reduces boilerplate for common list endpoint patterns.
///
/// # Example
///
/// ```rust
/// #[list_endpoint(
///     path = "/api/v1/pharmacy/pharmacies",
///     response = Pharmacy,
///     params = ListPharmaciesParams,
///     tag = "pharmacy"
/// )]
/// pub async fn list_pharmacies(...) -> ... { }
/// ```
///
/// Expands to:
/// ```rust
/// #[utoipa::path(
///     get,
///     path = "/api/v1/pharmacy/pharmacies",
///     responses(
///         (status = 200, description = "List retrieved successfully", body = Vec<Pharmacy>),
///         (status = 401, description = "Unauthorized"),
///         (status = 500, description = "Internal server error")
///     ),
///     params(ListPharmaciesParams),
///     tag = "pharmacy",
///     security(("bearer_auth" = []))
/// )]
/// ```
#[macro_export]
macro_rules! list_endpoint {
    (
        path = $path:literal,
        response = $response:ty,
        params = $params:ty,
        tag = $tag:literal
    ) => {
        #[utoipa::path(
            get,
            path = $path,
            responses(
                (status = 200, description = "List retrieved successfully", body = Vec<$response>),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            params($params),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
}

/// Generate utoipa::path attribute for a get-by-id endpoint
///
/// # Example
///
/// ```rust
/// #[get_endpoint(
///     path = "/api/v1/pharmacy/pharmacies/{pharmacy_id}",
///     response = Pharmacy,
///     id_param = ("pharmacy_id", Uuid, "Pharmacy ID"),
///     tag = "pharmacy"
/// )]
/// pub async fn get_pharmacy(...) -> ... { }
/// ```
#[macro_export]
macro_rules! get_endpoint {
    (
        path = $path:literal,
        response = $response:ty,
        id_param = ($id_name:literal, $id_type:ty, $id_desc:literal),
        tag = $tag:literal
    ) => {
        #[utoipa::path(
            get,
            path = $path,
            responses(
                (status = 200, description = "Resource retrieved successfully", body = $response),
                (status = 404, description = "Resource not found"),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            params(
                ($id_name = $id_type, Path, description = $id_desc)
            ),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
}

/// Generate utoipa::path attribute for a create endpoint
///
/// # Example
///
/// ```rust
/// #[create_endpoint(
///     path = "/api/v1/pharmacy/pharmacies",
///     request = CreatePharmacyRequest,
///     response = Pharmacy,
///     tag = "pharmacy"
/// )]
/// pub async fn create_pharmacy(...) -> ... { }
/// ```
#[macro_export]
macro_rules! create_endpoint {
    (
        path = $path:literal,
        request = $request:ty,
        response = $response:ty,
        tag = $tag:literal
    ) => {
        #[utoipa::path(
            post,
            path = $path,
            request_body = $request,
            responses(
                (status = 201, description = "Resource created successfully", body = $response),
                (status = 400, description = "Invalid request"),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
}

/// Generate utoipa::path attribute for an update endpoint
///
/// # Example
///
/// ```rust
/// #[update_endpoint(
///     path = "/api/v1/pharmacy/pharmacies/{pharmacy_id}",
///     request = UpdatePharmacyRequest,
///     response = Pharmacy,
///     id_param = ("pharmacy_id", Uuid, "Pharmacy ID"),
///     tag = "pharmacy"
/// )]
/// pub async fn update_pharmacy(...) -> ... { }
/// ```
#[macro_export]
macro_rules! update_endpoint {
    (
        path = $path:literal,
        request = $request:ty,
        response = $response:ty,
        id_param = ($id_name:literal, $id_type:ty, $id_desc:literal),
        tag = $tag:literal
    ) => {
        #[utoipa::path(
            put,
            path = $path,
            request_body = $request,
            responses(
                (status = 200, description = "Resource updated successfully", body = $response),
                (status = 404, description = "Resource not found"),
                (status = 400, description = "Invalid request"),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            params(
                ($id_name = $id_type, Path, description = $id_desc)
            ),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
}

/// Generate utoipa::path attribute for a delete endpoint
///
/// # Example
///
/// ```rust
/// #[delete_endpoint(
///     path = "/api/v1/pharmacy/pharmacies/{pharmacy_id}",
///     id_param = ("pharmacy_id", Uuid, "Pharmacy ID"),
///     tag = "pharmacy"
/// )]
/// pub async fn delete_pharmacy(...) -> ... { }
/// ```
#[macro_export]
macro_rules! delete_endpoint {
    (
        path = $path:literal,
        id_param = ($id_name:literal, $id_type:ty, $id_desc:literal),
        tag = $tag:literal
    ) => {
        #[utoipa::path(
            delete,
            path = $path,
            responses(
                (status = 204, description = "Resource deleted successfully"),
                (status = 404, description = "Resource not found"),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            params(
                ($id_name = $id_type, Path, description = $id_desc)
            ),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
}

/// Generate utoipa::path attribute for a custom endpoint
///
/// This macro provides a flexible way to generate utoipa paths with custom configurations.
/// Use this when the standard macros don't fit your needs.
///
/// # Example
///
/// ```rust
/// #[custom_endpoint(
///     method = post,
///     path = "/api/v1/pharmacy/pharmacies/{pharmacy_id}/activate",
///     response = Pharmacy,
///     id_param = ("pharmacy_id", Uuid, "Pharmacy ID"),
///     tag = "pharmacy",
///     description = "Activate a pharmacy"
/// )]
/// pub async fn activate_pharmacy(...) -> ... { }
/// ```
#[macro_export]
macro_rules! custom_endpoint {
    (
        method = $method:ident,
        path = $path:literal,
        response = $response:ty,
        id_param = ($id_name:literal, $id_type:ty, $id_desc:literal),
        tag = $tag:literal,
        description = $desc:literal
    ) => {
        #[utoipa::path(
            $method,
            path = $path,
            responses(
                (status = 200, description = $desc, body = $response),
                (status = 404, description = "Resource not found"),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            params(
                ($id_name = $id_type, Path, description = $id_desc)
            ),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
    (
        method = $method:ident,
        path = $path:literal,
        request = $request:ty,
        response = $response:ty,
        tag = $tag:literal,
        description = $desc:literal
    ) => {
        #[utoipa::path(
            $method,
            path = $path,
            request_body = $request,
            responses(
                (status = 200, description = $desc, body = $response),
                (status = 400, description = "Invalid request"),
                (status = 401, description = "Unauthorized"),
                (status = 500, description = "Internal server error")
            ),
            tag = $tag,
            security(("bearer_auth" = []))
        )]
    };
}

