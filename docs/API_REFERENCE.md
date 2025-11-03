# RustCare Engine - Complete API Reference

**Last Updated**: 2025-01-30  
**API Base Path**: `/api/v1` (unless otherwise specified)

This document lists ALL existing API endpoints in the RustCare Engine codebase to prevent duplicate implementation.

---

## Table of Contents

1. [Health & System](#health--system)
2. [Authentication](#authentication)
3. [Organizations](#organizations)
4. [Onboarding](#onboarding)
5. [Permissions & Authorization](#permissions--authorization)
6. [Healthcare](#healthcare)
7. [Pharmacy](#pharmacy)
8. [Vendors](#vendors)
9. [Devices](#devices)
10. [Geographic](#geographic)
11. [Compliance](#compliance)
12. [Notifications](#notifications)
13. [Workflow](#workflow)
14. [Sync](#sync)
15. [Secrets Management](#secrets-management)
16. [KMS (Key Management Service)](#kms-key-management-service)
17. [WebSocket](#websocket)

---

## Health & System

**Base Path**: `/` (root level)

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/health` | `health::health_check` | ✅ | Health check endpoint |
| GET | `/version` | `health::version_info` | ✅ | Version information |
| GET | `/status` | `health::system_status` | ✅ | System status |

---

## Authentication

**Base Path**: `/api/v1/auth`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/auth/login` | `auth::login` | ✅ | Email/password, OAuth, certificate auth |
| POST | `/api/v1/auth/logout` | `auth::logout` | ✅ | Session termination |
| POST | `/api/v1/auth/oauth/authorize` | `auth::oauth_authorize` | ✅ | OAuth authorization |
| POST | `/api/v1/auth/token/validate` | `auth::validate_token` | ✅ | JWT token validation |

---

## Organizations

**Base Path**: `/api/v1/organizations`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/organizations` | `organizations::list_organizations` | ✅ | List all organizations |
| POST | `/api/v1/organizations` | `organizations::create_organization` | ✅ | Create new organization |
| GET | `/api/v1/role-templates` | `organizations::get_role_templates` | ✅ | Get role templates |
| GET | `/api/v1/organizations/{org_id}/roles` | `organizations::list_organization_roles` | ✅ | List organization roles |
| POST | `/api/v1/organizations/{org_id}/roles` | `organizations::create_organization_role` | ✅ | Create organization role |
| GET | `/api/v1/organizations/{org_id}/employees` | `organizations::list_organization_employees` | ✅ | List employees |
| POST | `/api/v1/organizations/{org_id}/employees/{employee_id}/roles` | `organizations::assign_employee_role` | ✅ | Assign role to employee |
| GET | `/api/v1/organizations/{org_id}/patients` | `organizations::list_organization_patients` | ✅ | List patients |
| POST | `/api/v1/organizations/{org_id}/patients` | `organizations::create_organization_patient` | ✅ | Create patient |

---

## Onboarding

**Base Path**: `/api/v1/organizations/{org_id}`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/organizations/{org_id}/users` | `onboarding::create_organization_user` | ✅ | Create organization user |
| GET | `/api/v1/organizations/{org_id}/users` | `onboarding::list_organization_users` | ✅ | List organization users |
| POST | `/api/v1/users/{user_id}/resend-credentials` | `onboarding::resend_user_credentials` | ✅ | Resend user credentials |
| POST | `/api/v1/email/verify` | `onboarding::verify_email_config` | ✅ | Verify email configuration |

---

## Permissions & Authorization

**Base Path**: `/api/permissions`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/permissions/resources` | `permissions::list_resources` | ✅ | List all resources |
| POST | `/api/permissions/resources` | `permissions::create_resource` | ✅ | Create new resource |
| GET | `/api/permissions/groups` | `permissions::list_groups` | ✅ | List permission groups |
| POST | `/api/permissions/groups` | `permissions::create_group` | ✅ | Create permission group |
| GET | `/api/permissions/roles` | `permissions::list_roles` | ✅ | List all roles |
| POST | `/api/permissions/roles` | `permissions::create_role` | ✅ | Create new role |
| GET | `/api/permissions/users/{user_id}` | `permissions::get_user_permissions` | ✅ | Get user permissions |
| PUT | `/api/permissions/users/{user_id}` | `permissions::assign_user_permissions` | ✅ | Assign permissions to user |
| POST | `/api/auth/check` | `permissions::check_permission` | ✅ | Check if user has permission |

---

## Healthcare

**Base Path**: `/api/v1/healthcare`

### Medical Records

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/healthcare/medical-records` | `healthcare::list_medical_records` | ✅ | List with filters (patient_id, provider_id, record_type, dates) |
| POST | `/api/v1/healthcare/medical-records` | `healthcare::create_medical_record` | ✅ | Create new medical record |
| GET | `/api/v1/healthcare/medical-records/{record_id}` | `healthcare::get_medical_record` | ✅ | Get specific record |
| PUT | `/api/v1/healthcare/medical-records/{record_id}` | `healthcare::update_medical_record` | ✅ | Update medical record |
| DELETE | `/api/v1/healthcare/medical-records/{record_id}` | `healthcare::delete_medical_record` | ✅ | Soft delete record |
| GET | `/api/v1/healthcare/medical-records/{record_id}/audit` | `healthcare::get_medical_record_audit` | ✅ | Get audit log for record |

### Providers

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/healthcare/providers` | `healthcare::list_providers` | ✅ | List healthcare providers |

### Service Types

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/healthcare/service-types` | `healthcare::list_service_types` | ✅ | List service types |
| POST | `/api/v1/healthcare/service-types` | `healthcare::create_service_type` | ✅ | Create service type |
| GET | `/api/v1/healthcare/service-types/{service_type_id}` | `healthcare::get_service_type` | ✅ | Get specific service type |
| PUT | `/api/v1/healthcare/service-types/{service_type_id}` | `healthcare::update_service_type` | ✅ | Update service type |
| DELETE | `/api/v1/healthcare/service-types/{service_type_id}` | `healthcare::delete_service_type` | ✅ | Delete service type |

### Appointments

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/healthcare/appointments` | `healthcare::list_appointments` | ✅ | List appointments with filters |
| POST | `/api/v1/healthcare/appointments` | `healthcare::create_appointment` | ✅ | Create new appointment |
| PUT | `/api/v1/healthcare/appointments/{appointment_id}/status` | `healthcare::update_appointment_status` | ✅ | Update appointment status |

**Note**: Database tables exist for:
- `appointments` ✅
- `provider_availability` ✅
- `patient_visits` ✅
- `clinical_orders` ✅
- `order_results` ✅
- `vital_signs` ✅

But API endpoints are **missing** for:
- ❌ Visits/Encounters (POST, GET, PUT, DELETE)
- ❌ Provider Availability (GET, POST, PUT, DELETE)
- ❌ Clinical Orders (GET, POST, PUT, DELETE)
- ❌ Order Results (GET, POST, PUT)
- ❌ Vital Signs (GET, POST, PUT, DELETE)

---

## Pharmacy

**Base Path**: `/api/v1/pharmacy`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/pharmacy/pharmacies` | `pharmacy::list_pharmacies` | ⚠️ | Returns empty array (TODO: implement DB query) |
| GET | `/api/v1/pharmacy/inventory` | `pharmacy::list_inventory` | ⚠️ | Returns empty array (TODO: implement DB query) |
| GET | `/api/v1/pharmacy/prescriptions` | `pharmacy::list_prescriptions` | ⚠️ | Returns empty array (TODO: implement DB query) |

**Database Tables Exist**:
- ✅ `pharmacies`
- ✅ `medications`
- ✅ `pharmacy_inventory`
- ✅ `prescriptions`

**Missing API Endpoints**:
- ❌ POST `/api/v1/pharmacy/pharmacies` (create pharmacy)
- ❌ GET/PUT/DELETE `/api/v1/pharmacy/pharmacies/{id}` (CRUD operations)
- ❌ GET/POST `/api/v1/pharmacy/medications` (medication catalog)
- ❌ GET/POST `/api/v1/pharmacy/inventory/{id}` (inventory operations)
- ❌ POST `/api/v1/pharmacy/prescriptions` (create prescription)
- ❌ GET/PUT `/api/v1/pharmacy/prescriptions/{id}` (prescription operations)

---

## Vendors

**Base Path**: `/api/v1/vendors`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/vendors/types` | `vendors::list_vendor_types` | ⚠️ | Returns mock data |
| GET | `/api/v1/vendors` | `vendors::list_vendors` | ⚠️ | Returns empty array (TODO: implement DB query) |
| GET | `/api/v1/vendors/{vendor_id}/inventory` | `vendors::get_vendor_inventory` | ⚠️ | Returns empty array (TODO: implement DB query) |
| GET | `/api/v1/vendors/{vendor_id}/services` | `vendors::get_vendor_services` | ⚠️ | Returns empty array (TODO: implement DB query) |

**Database Tables Exist**:
- ✅ `vendor_types`
- ✅ `vendors`
- ✅ `vendor_inventory`
- ✅ `vendor_services`
- ✅ `purchase_orders`
- ✅ `purchase_order_items`
- ✅ `vendor_contracts`

**Missing API Endpoints**:
- ❌ POST `/api/v1/vendors/types` (create vendor type)
- ❌ POST `/api/v1/vendors` (create vendor)
- ❌ GET/PUT/DELETE `/api/v1/vendors/{id}` (vendor CRUD)
- ❌ POST `/api/v1/vendors/{vendor_id}/inventory` (add inventory item)
- ❌ POST `/api/v1/vendors/{vendor_id}/services` (add service)
- ❌ GET/POST `/api/v1/vendors/{vendor_id}/purchase-orders` (PO management)
- ❌ GET/POST `/api/v1/vendors/{vendor_id}/contracts` (contract management)

---

## Devices

**Base Path**: `/api/v1/devices`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/devices` | `devices::list_devices` | ✅ | List devices with filters |
| POST | `/api/v1/devices` | `devices::register_device` | ✅ | Register new device |
| GET | `/api/v1/devices/{device_id}` | `devices::get_device` | ✅ | Get device details |
| PUT | `/api/v1/devices/{device_id}` | `devices::update_device` | ✅ | Update device |
| DELETE | `/api/v1/devices/{device_id}` | `devices::delete_device` | ✅ | Delete device |
| POST | `/api/v1/devices/{device_id}/connect` | `devices::connect_device` | ✅ | Connect device |
| POST | `/api/v1/devices/{device_id}/disconnect` | `devices::disconnect_device` | ✅ | Disconnect device |
| GET | `/api/v1/devices/{device_id}/data` | `devices::get_device_data_history` | ✅ | Get device data history |
| POST | `/api/v1/devices/{device_id}/data/read` | `devices::read_device_data` | ✅ | Read device data |
| GET | `/api/v1/devices/{device_id}/commands` | `devices::get_device_commands` | ✅ | List available commands |
| POST | `/api/v1/devices/{device_id}/commands` | `devices::send_device_command` | ✅ | Send command to device |
| GET | `/api/v1/devices/types` | `devices::list_device_types` | ✅ | List device types |
| GET | `/api/v1/devices/connection-types` | `devices::list_connection_types` | ✅ | List connection types |
| GET | `/api/v1/devices/formats` | `devices::list_data_formats` | ✅ | List data formats |

---

## Geographic

**Base Path**: `/api/v1/geographic`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/geographic/regions` | `geographic::list_geographic_regions` | ✅ | List geographic regions |
| POST | `/api/v1/geographic/regions` | `geographic::create_geographic_region` | ✅ | Create geographic region |
| GET | `/api/v1/geographic/regions/{id}` | `geographic::get_geographic_region` | ✅ | Get specific region |
| PUT | `/api/v1/geographic/regions/{id}` | `geographic::update_geographic_region` | ✅ | Update region |
| DELETE | `/api/v1/geographic/regions/{id}` | `geographic::delete_geographic_region` | ✅ | Delete region |
| GET | `/api/v1/geographic/regions/{id}/hierarchy` | `geographic::get_geographic_hierarchy` | ✅ | Get region hierarchy |
| GET | `/api/v1/geographic/postal-codes/{postal_code}/compliance` | `geographic::get_postal_code_compliance` | ✅ | Get compliance for postal code |

---

## Compliance

**Base Path**: `/api/v1/compliance`

### Frameworks

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/compliance/frameworks` | `compliance::list_frameworks` | ✅ | List compliance frameworks |
| POST | `/api/v1/compliance/frameworks` | `compliance::create_framework` | ✅ | Create framework |
| GET | `/api/v1/compliance/frameworks/{id}` | `compliance::get_framework` | ✅ | Get framework details |
| PUT | `/api/v1/compliance/frameworks/{id}` | `compliance::update_framework` | ✅ | Update framework |
| DELETE | `/api/v1/compliance/frameworks/{id}` | `compliance::delete_framework` | ✅ | Delete framework |
| GET | `/api/v1/compliance/frameworks/{framework_id}/rules` | `compliance::list_framework_rules` | ✅ | List rules for framework |
| GET | `/api/v1/compliance/frameworks/{id}/rules` | `compliance::list_framework_rules` | ✅ | Alternative endpoint |

### Rules

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/compliance/rules` | `compliance::list_rules` | ✅ | List all compliance rules |
| POST | `/api/v1/compliance/rules` | `compliance::create_rule` | ✅ | Create compliance rule |
| GET | `/api/v1/compliance/rules/{id}` | `compliance::get_rule` | ✅ | Get rule details |
| PUT | `/api/v1/compliance/rules/{id}` | `compliance::update_rule` | ✅ | Update rule |
| DELETE | `/api/v1/compliance/rules/{id}` | `compliance::delete_rule` | ✅ | Delete rule |

### Entity Compliance

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/compliance/auto-assign` | `compliance::auto_assign_compliance` | ✅ | Auto-assign compliance |
| GET | `/api/v1/compliance/entities/{entity_type}/{entity_id}` | `compliance::get_entity_compliance` | ✅ | Get entity compliance |
| POST | `/api/v1/compliance/entities/{entity_type}/{entity_id}/assess` | `compliance::assess_entity_compliance` | ✅ | Assess entity compliance |

---

## Notifications

**Base Path**: `/api/v1/notifications`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/notifications` | `notifications::list_notifications` | ✅ | List notifications with filters |
| GET | `/api/v1/notifications/{id}` | `notifications::get_notification` | ✅ | Get specific notification |
| POST | `/api/v1/notifications` | `notifications::create_notification` | ✅ | Create notification |
| PUT | `/api/v1/notifications/{id}/read` | `notifications::mark_notification_read` | ✅ | Mark as read |
| POST | `/api/v1/notifications/bulk-read` | `notifications::bulk_mark_read` | ✅ | Bulk mark as read |
| GET | `/api/v1/notifications/unread/count` | `notifications::get_unread_count` | ✅ | Get unread count |
| GET | `/api/v1/notifications/{id}/audit-logs` | `notifications::list_audit_logs` | ✅ | Get notification audit logs |

---

## Workflow

**Base Path**: `/api/v1/workflow`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/workflow/workflows` | `workflow::list_workflows` | ⚠️ | Returns mock data |
| GET | `/api/v1/workflow/workflows/:id` | `workflow::get_workflow` | ⚠️ | Returns mock data |
| POST | `/api/v1/workflow/workflows/execute` | `workflow::execute_workflow` | ⚠️ | Mock implementation |
| GET | `/api/v1/workflow/executions/:id/status` | `workflow::get_execution_status` | ⚠️ | Mock implementation |
| DELETE | `/api/v1/workflow/executions/:id/cancel` | `workflow::cancel_execution` | ⚠️ | Mock implementation |

**Note**: Workflow engine exists but APIs return mock data.

---

## Sync

**Base Path**: `/api/v1/sync`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/sync/pull` | `sync::pull` | ✅ | Pull sync data |
| POST | `/api/v1/sync/push` | `sync::push` | ✅ | Push sync data |

---

## Secrets Management

**Base Path**: `/api/v1/secrets`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/secrets/health` | `secrets::secrets_health_check` | ✅ | Secrets service health |
| GET | `/api/v1/secrets` | `secrets::list_secrets` | ✅ | List all secrets |
| POST | `/api/v1/secrets` | `secrets::create_secret` | ✅ | Create secret |
| GET | `/api/v1/secrets/:key` | `secrets::get_secret` | ✅ | Get secret value |
| PUT | `/api/v1/secrets/:key` | `secrets::update_secret` | ✅ | Update secret |
| DELETE | `/api/v1/secrets/:key` | `secrets::delete_secret` | ✅ | Delete secret |
| GET | `/api/v1/secrets/:key/versions` | `secrets::list_secret_versions` | ✅ | List secret versions |
| GET | `/api/v1/secrets/:key/versions/:version` | `secrets::get_secret_version` | ✅ | Get specific version |
| POST | `/api/v1/secrets/:key/rotate` | `secrets::rotate_secret` | ✅ | Rotate secret |

---

## KMS (Key Management Service)

**Base Path**: `/api/v1/kms`

### Data Keys

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/kms/datakey/generate` | `kms::generate_data_key` | ✅ | Generate data key |
| POST | `/api/v1/kms/datakey/decrypt` | `kms::decrypt_data_key` | ✅ | Decrypt data key |

### Encryption Operations

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/kms/encrypt` | `kms::encrypt` | ✅ | Encrypt data |
| POST | `/api/v1/kms/decrypt` | `kms::decrypt` | ✅ | Decrypt data |
| POST | `/api/v1/kms/re-encrypt` | `kms::re_encrypt` | ✅ | Re-encrypt data |

### Key Management

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| GET | `/api/v1/kms/keys` | `kms::list_keys` | ✅ | List all keys |
| POST | `/api/v1/kms/keys` | `kms::create_key` | ✅ | Create new key |
| GET | `/api/v1/kms/keys/:key_id` | `kms::describe_key` | ✅ | Describe key |
| POST | `/api/v1/kms/keys/:key_id/enable` | `kms::enable_key` | ✅ | Enable key |
| POST | `/api/v1/kms/keys/:key_id/disable` | `kms::disable_key` | ✅ | Disable key |

### Key Rotation

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/kms/keys/:key_id/rotation/enable` | `kms::enable_key_rotation` | ✅ | Enable rotation |
| POST | `/api/v1/kms/keys/:key_id/rotation/disable` | `kms::disable_key_rotation` | ✅ | Disable rotation |
| GET | `/api/v1/kms/keys/:key_id/rotation/status` | `kms::get_key_rotation_status` | ✅ | Get rotation status |
| POST | `/api/v1/kms/keys/:key_id/rotate` | `kms::rotate_key` | ✅ | Rotate key |

### Key Deletion

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| POST | `/api/v1/kms/keys/:key_id/schedule-deletion` | `kms::schedule_key_deletion` | ✅ | Schedule deletion |
| POST | `/api/v1/kms/keys/:key_id/cancel-deletion` | `kms::cancel_key_deletion` | ✅ | Cancel deletion |

---

## WebSocket

**Base Path**: `/ws`

| Method | Endpoint | Handler | Status | Notes |
|--------|----------|---------|--------|-------|
| WS | `/ws` | `websocket::handle_websocket` | ⚠️ | Temporarily disabled in routes |

---

## Summary: Missing APIs vs Existing Tables

### ❌ **Missing APIs** (Database tables exist, but no API endpoints):

1. **Healthcare**
   - ❌ Patient Visits/Encounters (CRUD)
   - ❌ Provider Availability (CRUD)
   - ❌ Clinical Orders (CRUD)
   - ❌ Order Results (CRUD)
   - ❌ Vital Signs (CRUD)

2. **Pharmacy**
   - ❌ Pharmacies (POST, PUT, DELETE, GET by ID)
   - ❌ Medications (CRUD)
   - ❌ Pharmacy Inventory (full CRUD)
   - ❌ Prescriptions (POST, PUT, GET by ID)

3. **Vendors**
   - ❌ Vendor Types (POST, PUT, DELETE)
   - ❌ Vendors (POST, PUT, DELETE, GET by ID)
   - ❌ Vendor Inventory (POST, PUT, DELETE)
   - ❌ Vendor Services (POST, PUT, DELETE)
   - ❌ Purchase Orders (full CRUD)
   - ❌ Vendor Contracts (full CRUD)

4. **Facility Management** (NEW - no tables yet)
   - ❌ Facilities
   - ❌ Rooms
   - ❌ Beds

5. **Fleet/Ambulance** (NEW - no tables yet)
   - ❌ Vehicles
   - ❌ Ambulances
   - ❌ Vehicle Maintenance
   - ❌ Ambulance Trips

6. **Parking** (NEW - no tables yet)
   - ❌ Parking Lots
   - ❌ Parking Spaces
   - ❌ Parking Reservations

### ✅ **Complete APIs** (Both database tables AND endpoints exist):

- ✅ Authentication
- ✅ Organizations
- ✅ Permissions
- ✅ Devices
- ✅ Geographic
- ✅ Compliance
- ✅ Notifications
- ✅ Secrets Management
- ✅ KMS

---

## Notes

1. **Status Legend**:
   - ✅ = Fully implemented
   - ⚠️ = Partially implemented (returns mock/empty data)
   - ❌ = Not implemented

2. **Common Patterns**:
   - All APIs use `/api/v1/` prefix (except health, permissions use `/api/`)
   - Most endpoints follow RESTful conventions
   - Response format: `ApiResponse<T>` wrapper
   - Error format: `ApiError` enum

3. **Authentication**:
   - Most endpoints require Bearer token authentication
   - JWT tokens are used for stateless authentication
   - RLS (Row-Level Security) is enforced at database level

---

**Document Version**: 1.0  
**Generated**: 2025-01-30

