# Phase 9: Configurable RLS+Zanzibar Field Masking

## Overview

This phase builds upon the static field masking (Task 15) to create a **dynamic, configurable masking system** that integrates with RLS context and Zanzibar authorization for relationship-based access control.

## Architectural Evolution

### Current State (Task 15 - ✅ Complete)
- Static permission-based masking (`phi:view:{level}`)
- Fixed masking patterns per field
- No relationship awareness
- Organization-agnostic policies

### Target State (Phase 9)
- Dynamic relationship-based masking
- Configurable policies per organization
- Zanzibar integration for fine-grained checks
- Time-bound emergency access
- Audit trail with decision context

---

## Architecture Layers

### Layer 1: Field Classification Service

**Purpose**: Central catalog of all sensitive fields with configurable policies.

**Components**:

1. **Field Registry** - Database-backed catalog of sensitive fields
2. **Sensitivity Metadata** - Per-field configuration:
   - Base sensitivity level (Public → ePHI)
   - Required permissions to view unmasked
   - Encryption requirements
   - Audit logging requirements
   - Time-to-live for access
   - Emergency access rules

**Database Schema**:

```sql
-- Configuration table for masking policies
CREATE TABLE masking_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id),
    field_path VARCHAR(255) NOT NULL, -- Format: "table.column"
    
    -- Classification
    sensitivity_level VARCHAR(50) NOT NULL, -- Public, Internal, Confidential, Restricted, ProtectedHealthInfo
    base_mask_pattern JSONB NOT NULL, -- e.g., {"type": "Partial", "show_first": 0, "show_last": 4}
    
    -- Security requirements
    encryption_required BOOLEAN DEFAULT FALSE,
    audit_all_access BOOLEAN DEFAULT TRUE,
    audit_retention_days INTEGER DEFAULT 2555, -- 7 years HIPAA
    
    -- Permission requirements (static fallback)
    unmasked_permissions TEXT[] NOT NULL DEFAULT '{}',
    partial_permissions TEXT[] DEFAULT '{}',
    redacted_permissions TEXT[] DEFAULT '{}',
    
    -- Zanzibar checks (dynamic evaluation)
    zanzibar_checks JSONB DEFAULT '[]', -- Array of {relation, object_type, mask_pattern}
    
    -- Time constraints
    time_constraints JSONB, -- {allowed_hours: [9-17], allowed_days: [1-5], timezone: "UTC"}
    ip_whitelist TEXT[],
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    is_active BOOLEAN DEFAULT TRUE,
    
    CONSTRAINT masking_policies_org_field_unique UNIQUE(organization_id, field_path)
);

-- Indexes for performance
CREATE INDEX idx_masking_policies_org ON masking_policies(organization_id) WHERE is_active = TRUE;
CREATE INDEX idx_masking_policies_field ON masking_policies(field_path) WHERE is_active = TRUE;
CREATE INDEX idx_masking_policies_sensitivity ON masking_policies(sensitivity_level) WHERE is_active = TRUE;

-- Temporary overrides for emergency access
CREATE TABLE masking_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID REFERENCES masking_policies(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id),
    organization_id UUID REFERENCES organizations(id),
    
    -- Override details
    override_type VARCHAR(50) NOT NULL, -- 'emergency', 'admin', 'break_glass', 'audit_review'
    new_mask_pattern JSONB, -- Override pattern or NULL for unmasked
    reason TEXT NOT NULL,
    justification TEXT,
    
    -- Time bounds (auto-expire)
    valid_from TIMESTAMPTZ DEFAULT NOW(),
    valid_until TIMESTAMPTZ NOT NULL,
    
    -- Approval workflow
    requested_by UUID REFERENCES users(id),
    approved_by UUID REFERENCES users(id),
    approval_status VARCHAR(50) DEFAULT 'pending', -- pending, approved, denied, expired
    approved_at TIMESTAMPTZ,
    
    -- Audit
    access_count INTEGER DEFAULT 0,
    last_access_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT masking_overrides_valid_time CHECK (valid_until > valid_from),
    CONSTRAINT masking_overrides_max_duration CHECK (valid_until <= valid_from + INTERVAL '8 hours')
);

CREATE INDEX idx_masking_overrides_user ON masking_overrides(user_id, valid_from, valid_until) WHERE approval_status = 'approved';
CREATE INDEX idx_masking_overrides_expiry ON masking_overrides(valid_until) WHERE approval_status = 'approved';
CREATE INDEX idx_masking_overrides_org ON masking_overrides(organization_id, approval_status);

-- Audit log for masking decisions
CREATE TABLE masking_decisions_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Context
    user_id UUID NOT NULL REFERENCES users(id),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    session_id VARCHAR(255),
    
    -- Field access
    field_path VARCHAR(255) NOT NULL,
    record_id UUID,
    
    -- Decision details
    mask_pattern_applied VARCHAR(50) NOT NULL, -- None, Partial, Full, Redacted, Hashed, Tokenized
    decision_reason TEXT NOT NULL, -- e.g., "static_permission: phi:view:ephi", "zanzibar: user is assigned_physician"
    
    -- Evaluation chain
    static_permission_check JSONB, -- {passed: true, permissions: [...]}
    zanzibar_checks JSONB, -- Array of {relation, object, result, latency_ms}
    time_constraint_check JSONB,
    override_applied UUID REFERENCES masking_overrides(id),
    
    -- Performance
    evaluation_time_ms INTEGER,
    
    -- Metadata
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT
);

-- Partitioned by month for performance
CREATE INDEX idx_masking_decisions_user_time ON masking_decisions_audit(user_id, timestamp DESC);
CREATE INDEX idx_masking_decisions_field ON masking_decisions_audit(field_path, timestamp DESC);
CREATE INDEX idx_masking_decisions_org ON masking_decisions_audit(organization_id, timestamp DESC);
```

**Example Configuration**:

```json
{
  "field_path": "patient.diagnosis",
  "sensitivity_level": "ProtectedHealthInfo",
  "base_mask_pattern": {
    "type": "Redacted"
  },
  "encryption_required": true,
  "audit_all_access": true,
  "unmasked_permissions": ["phi:view:ephi", "admin:*"],
  "partial_permissions": ["phi:view:confidential"],
  "zanzibar_checks": [
    {
      "relation": "assigned_physician",
      "object_type": "patient",
      "mask_pattern": "None",
      "description": "Assigned physician sees unmasked diagnosis"
    },
    {
      "relation": "care_team_member",
      "object_type": "patient",
      "mask_pattern": "Partial",
      "description": "Care team sees first word only"
    }
  ],
  "time_constraints": {
    "allowed_hours": [6, 22],
    "allowed_days": [1, 2, 3, 4, 5, 6, 7],
    "timezone": "UTC"
  }
}
```

---

### Layer 2: RLS Context Integration

**Purpose**: Organization-level boundaries + organization-specific policies.

**Key Concepts**:

1. **RLS filters records** (which rows user can see)
2. **Masking protects fields** (what data within rows is visible)
3. **Organizations customize policies** (per-tenant configuration)

**RlsContext Enhancement**:

```rust
pub struct RlsContext {
    pub user_id: Uuid,
    pub tenant_id: String,
    pub organization_id: Option<Uuid>, // NEW: For org-specific policies
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}
```

**Organization Policy Overrides**:

```rust
// Example: Hospital A requires stricter masking than Hospital B
// Hospital A (trauma center)
{
  "organization_id": "org-hospital-a",
  "field_path": "patient.ssn",
  "base_mask_pattern": {"type": "Full"}, // Show NO digits
  "unmasked_permissions": ["admin:*", "compliance:*"]
}

// Hospital B (research facility)
{
  "organization_id": "org-hospital-b",
  "field_path": "patient.ssn",
  "base_mask_pattern": {"type": "Partial", "show_first": 0, "show_last": 4},
  "unmasked_permissions": ["admin:*", "compliance:*", "research:approved"]
}
```

---

### Layer 3: Zanzibar Authorization Integration

**Purpose**: Relationship-based fine-grained masking decisions.

**Zanzibar Healthcare Schema**:

```yaml
# Patient resource with relationships
patient:
  relations:
    owner: [user]                    # Patient themselves
    assigned_physician: [user]        # Primary care provider
    consulting_physician: [user]      # Specialist
    care_team_member: [user]         # Nurses, therapists
    department_viewer: [department]   # Department-level access
    emergency_responder: [user]       # ER staff with time-limited access
    
  permissions:
    view_full: owner + assigned_physician + admin
    view_clinical: consulting_physician + care_team_member
    view_demographics: department_viewer
    view_emergency: emergency_responder

# Medical record with field-level permissions
medical_record:
  relations:
    patient: [patient]
    viewer: [user]
    
  permissions:
    view_diagnosis: patient->view_full + patient->view_clinical
    view_ssn: patient->view_full + admin
    view_basic: patient->view_demographics

# Field-level resource (most granular)
field:
  relations:
    record: [medical_record]
    
  permissions:
    view_unmasked: record->patient->assigned_physician + admin
    view_partial: record->patient->care_team_member
    view_redacted: record->patient->department_viewer
```

**Masking Decision Flow with Zanzibar**:

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Request to access patient.diagnosis                      │
│    User: nurse-jones, Patient: patient-123                  │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Load Field Policy from masking_policies table            │
│    field_path: "patient.diagnosis"                          │
│    sensitivity: ProtectedHealthInfo                         │
│    zanzibar_checks: [assigned_physician, care_team_member]  │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Check Static Permissions (Fast Path)                     │
│    nurse-jones has: [phi:view:internal, phi:view:ephi]     │
│    Required: [phi:view:ephi]                                │
│    Result: ✅ PASS (but check Zanzibar for finer control)  │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Check Zanzibar Relationships                             │
│    Check 1: zanzibar.check(                                 │
│       subject: "user:nurse-jones",                          │
│       relation: "assigned_physician",                       │
│       object: "patient:patient-123"                         │
│    ) → Result: ❌ DENIED                                    │
│                                                             │
│    Check 2: zanzibar.check(                                 │
│       subject: "user:nurse-jones",                          │
│       relation: "care_team_member",                         │
│       object: "patient:patient-123"                         │
│    ) → Result: ✅ ALLOWED                                   │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Apply Masking Pattern                                    │
│    Relationship: care_team_member                           │
│    Pattern: Partial (first word only)                       │
│    Original: "Type 2 Diabetes Mellitus"                     │
│    Masked: "Type 2 ███████ ████████"                        │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ 6. Audit Decision                                           │
│    Log to masking_decisions_audit:                          │
│    - User: nurse-jones                                      │
│    - Field: patient.diagnosis                               │
│    - Pattern: Partial                                       │
│    - Reason: "zanzibar: care_team_member"                   │
│    - Latency: 12ms                                          │
└─────────────────────────────────────────────────────────────┘
```

---

### Layer 4: Dynamic Masking Policy Engine

**Purpose**: Runtime evaluation engine that makes masking decisions.

**Core Algorithm**:

```rust
pub struct MaskingEngine {
    policy_store: Arc<MaskingPolicyStore>,
    zanzibar_client: Arc<ZanzibarClient>,
    cache: Arc<RwLock<MaskingDecisionCache>>,
}

impl MaskingEngine {
    pub async fn mask_field_value(
        &self,
        field_path: &str,
        value: &str,
        record_id: &Uuid,
        rls_context: &RlsContext,
        request_context: &RequestContext,
    ) -> Result<MaskedValue> {
        let start = Instant::now();
        
        // 1. Load policy (with org-specific overrides)
        let policy = self.policy_store
            .get_policy(field_path, rls_context.organization_id)
            .await?;
        
        // 2. Check cache first (for performance)
        let cache_key = format!("{}:{}:{}", 
            rls_context.user_id, field_path, record_id);
        if let Some(cached) = self.cache.read().await.get(&cache_key) {
            if !cached.is_expired() {
                return Ok(cached.value.clone());
            }
        }
        
        // 3. Evaluate decision chain
        let decision = self.evaluate_masking_decision(
            &policy,
            record_id,
            rls_context,
            request_context,
        ).await?;
        
        // 4. Apply mask pattern
        let masked_value = self.apply_mask_pattern(
            value,
            &decision.mask_pattern,
        );
        
        // 5. Cache decision (1 minute TTL)
        self.cache.write().await.insert(
            cache_key,
            CachedDecision::new(masked_value.clone(), Duration::minutes(1)),
        );
        
        // 6. Audit decision
        self.audit_decision(
            field_path,
            record_id,
            rls_context,
            &decision,
            start.elapsed(),
        ).await?;
        
        Ok(masked_value)
    }
    
    async fn evaluate_masking_decision(
        &self,
        policy: &MaskingPolicy,
        record_id: &Uuid,
        rls_context: &RlsContext,
        request_context: &RequestContext,
    ) -> Result<MaskingDecision> {
        // Check 1: Emergency override (highest priority)
        if let Some(override_pattern) = self.check_emergency_override(
            policy,
            rls_context.user_id,
            record_id,
        ).await? {
            return Ok(MaskingDecision {
                mask_pattern: override_pattern,
                reason: "emergency_override".to_string(),
                zanzibar_checks: vec![],
            });
        }
        
        // Check 2: Time constraints
        if !policy.is_time_allowed(request_context.timestamp) {
            return Ok(MaskingDecision {
                mask_pattern: MaskPattern::Redacted,
                reason: "time_constraint_failed".to_string(),
                zanzibar_checks: vec![],
            });
        }
        
        // Check 3: Static permissions (fast path)
        if self.has_static_permission(
            &rls_context.permissions,
            &policy.unmasked_permissions,
        ) {
            // User has base permission, but check Zanzibar for finer control
            // This prevents over-privileged static permissions
        }
        
        // Check 4: Zanzibar relationship checks (fine-grained)
        for zanzibar_check in &policy.zanzibar_checks {
            let object = format!("{}:{}", zanzibar_check.object_type, record_id);
            
            let allowed = self.zanzibar_client.check(
                format!("user:{}", rls_context.user_id),
                &zanzibar_check.relation,
                &object,
            ).await?;
            
            if allowed {
                return Ok(MaskingDecision {
                    mask_pattern: zanzibar_check.mask_pattern.clone(),
                    reason: format!("zanzibar: {}", zanzibar_check.relation),
                    zanzibar_checks: vec![ZanzibarCheckResult {
                        relation: zanzibar_check.relation.clone(),
                        object: object.clone(),
                        allowed: true,
                        latency_ms: /* ... */,
                    }],
                });
            }
        }
        
        // Check 5: Fallback to partial permissions
        if self.has_static_permission(
            &rls_context.permissions,
            &policy.partial_permissions,
        ) {
            return Ok(MaskingDecision {
                mask_pattern: MaskPattern::Partial { show_first: 1, show_last: 0 },
                reason: "static_permission: partial".to_string(),
                zanzibar_checks: vec![],
            });
        }
        
        // Check 6: Default to strictest masking
        Ok(MaskingDecision {
            mask_pattern: policy.base_mask_pattern.clone(),
            reason: "default_policy".to_string(),
            zanzibar_checks: vec![],
        })
    }
}
```

---

### Layer 5: Configuration Management

**Purpose**: Runtime configuration of masking policies.

**Components**:

1. **MaskingPolicyStore** - Loads/caches policies from database
2. **Policy Hierarchy** - Global defaults → Org overrides → Temporary overrides
3. **Admin API** - REST API for policy management
4. **Policy Validation** - Ensures policies don't conflict with compliance requirements

**Policy Loading Example**:

```rust
pub struct MaskingPolicyStore {
    pool: PgPool,
    cache: Arc<RwLock<HashMap<String, MaskingPolicy>>>,
}

impl MaskingPolicyStore {
    pub async fn get_policy(
        &self,
        field_path: &str,
        organization_id: Option<Uuid>,
    ) -> Result<MaskingPolicy> {
        // Try cache first
        let cache_key = format!("{}:{:?}", field_path, organization_id);
        if let Some(policy) = self.cache.read().await.get(&cache_key) {
            return Ok(policy.clone());
        }
        
        // Load from database with hierarchy
        let policy = if let Some(org_id) = organization_id {
            // Try org-specific policy first
            sqlx::query_as!(
                MaskingPolicy,
                r#"
                SELECT * FROM masking_policies
                WHERE field_path = $1
                  AND (organization_id = $2 OR organization_id IS NULL)
                  AND is_active = TRUE
                ORDER BY organization_id DESC NULLS LAST
                LIMIT 1
                "#,
                field_path,
                org_id,
            )
            .fetch_optional(&self.pool)
            .await?
        } else {
            // Load global default
            sqlx::query_as!(
                MaskingPolicy,
                r#"
                SELECT * FROM masking_policies
                WHERE field_path = $1
                  AND organization_id IS NULL
                  AND is_active = TRUE
                LIMIT 1
                "#,
                field_path,
            )
            .fetch_optional(&self.pool)
            .await?
        };
        
        // Cache for 5 minutes
        if let Some(ref p) = policy {
            self.cache.write().await.insert(cache_key, p.clone());
        }
        
        policy.ok_or(Error::PolicyNotFound)
    }
}
```

---

## Implementation Plan

### Phase 9.1: Foundation (3-4 days)
- [ ] Create database tables (masking_policies, masking_overrides, masking_decisions_audit)
- [ ] Build MaskingPolicyStore with caching
- [ ] Implement policy loading with org overrides
- [ ] Create migration to populate default policies from code

### Phase 9.2: Zanzibar Integration (4-5 days)
- [ ] Define healthcare Zanzibar schema
- [ ] Implement Zanzibar check integration in MaskingEngine
- [ ] Add relationship-based masking evaluation
- [ ] Create Zanzibar relationship sync mechanisms

### Phase 9.3: Dynamic Masking Engine (3-4 days)
- [ ] Build evaluation algorithm with decision chain
- [ ] Add caching for performance (decision cache)
- [ ] Implement emergency override checking
- [ ] Create masking decision audit logging

### Phase 9.4: Repository Integration (3-4 days)
- [ ] Update all repositories to use new MaskingEngine
- [ ] Add Zanzibar client to repository constructors
- [ ] Update find_by_id_masked methods
- [ ] Create helper methods for bulk masking

### Phase 9.5: API & Configuration (5-7 days)
- [ ] Build admin API for policy management
- [ ] Create emergency access request/approval workflow
- [ ] Implement policy validation (HIPAA compliance checks)
- [ ] Add audit log viewer with masking decisions

### Phase 9.6: Testing & Optimization (3-4 days)
- [ ] Unit tests for evaluation algorithm
- [ ] Integration tests with RLS + Zanzibar
- [ ] Performance testing and caching optimization
- [ ] Load testing with 1000+ concurrent requests

---

## Benefits

1. **Centralized Policy Management** - Single source of truth for masking rules
2. **Organization-Specific Policies** - Each tenant can customize masking levels
3. **Fine-Grained Control** - Relationship-based access (assigned doctor vs. consultant)
4. **Defense in Depth** - RLS (org isolation) + Masking (field protection) + Zanzibar (relationships)
5. **Audit Trail** - Every masking decision logged with full context
6. **Emergency Access** - Break-glass with time limits and approval workflow
7. **Performance** - Multi-level caching (policy cache, decision cache, Zanzibar cache)
8. **HIPAA Compliant** - Minimum necessary + audit trail + encryption
9. **Flexible** - Add new fields/rules without code changes
10. **Testable** - Clear decision algorithm with dependency injection

---

## Integration with Existing System

This phase **builds upon** Task 15 (static masking):

- Task 15 provides the **foundation** (MaskingEngine, patterns, audit logging)
- Phase 9 adds **dynamic evaluation** (RLS + Zanzibar integration)
- Backward compatible - existing code continues to work
- Progressive enhancement - start with static, migrate to dynamic

---

## Success Metrics

- [ ] Policy loading latency < 5ms (with cache)
- [ ] Masking decision latency < 20ms (including Zanzibar checks)
- [ ] Cache hit rate > 80% for repeated decisions
- [ ] Zero HIPAA compliance violations
- [ ] 100% masking decisions audited
- [ ] Emergency access auto-expires < 2 hours
- [ ] Policy changes propagate < 60 seconds

---

**This architecture provides production-ready, relationship-based field masking that integrates seamlessly with RLS and Zanzibar!**
