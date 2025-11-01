-- HIPAA Compliance Rules Seed Data
-- This file populates HIPAA-specific compliance rules based on the HIPAA Security Rule (45 CFR Part 164.302-318)
-- These rules form the basis for technical, administrative, and physical safeguards requirements

-- Variables for framework references
-- Note: In production, these IDs should be queried dynamically

-- Get HIPAA framework IDs
DO $$
DECLARE
    hipaa_id UUID;
    hipaa_privacy_id UUID;
    hipaa_security_id UUID;
    hitech_id UUID;
BEGIN
    -- Get framework IDs
    SELECT id INTO hipaa_id FROM compliance_frameworks WHERE code = 'HIPAA' LIMIT 1;
    SELECT id INTO hipaa_privacy_id FROM compliance_frameworks WHERE code = 'HIPAA-PRIVACY' LIMIT 1;
    SELECT id INTO hipaa_security_id FROM compliance_frameworks WHERE code = 'HIPAA-SECURITY' LIMIT 1;
    SELECT id INTO hitech_id FROM compliance_frameworks WHERE code = 'HITECH' LIMIT 1;

    -- Only proceed if frameworks exist
    IF hipaa_security_id IS NOT NULL THEN

        -- ============================================================================
        -- ADMINISTRATIVE SAFEGUARDS (45 CFR § 164.308)
        -- ============================================================================

        -- Security Management Process
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, check_frequency_days,
            effective_date, metadata, created_by
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(1)',
            'Security Management Process',
            'Implement policies and procedures to prevent, detect, contain, and correct security violations.',
            'administrative',
            'critical',
            'mandatory',
            '["patient_record", "system", "organization"]'::jsonb,
            '{"has_policy": true, "has_procedures": true, "requires_documentation": true}'::jsonb,
            'Develop comprehensive security management policy. Establish procedures for risk analysis, sanctions, and information system activity review.',
            false,
            90,
            '2005-04-20'::date,
            '{"section": "164.308(a)(1)", "category": "Administrative Safeguards"}'::jsonb,
            NULL
        ) ON CONFLICT DO NOTHING;

        -- Assigned Security Responsibility
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(2)',
            'Assigned Security Responsibility',
            'Identify the security official who is responsible for the development and implementation of security policies and procedures.',
            'administrative',
            'high',
            'mandatory',
            '["organization", "system"]'::jsonb,
            '{"has_security_officer": true, "has_documented_responsibilities": true}'::jsonb,
            'Designate a security officer. Document their responsibilities in writing.',
            false,
            '2005-04-20'::date,
            '{"section": "164.308(a)(2)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Workforce Security
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(3)',
            'Workforce Security',
            'Implement procedures for the authorization and/or supervision of workforce members who work with ePHI.',
            'administrative',
            'critical',
            'mandatory',
            '["user", "employee", "workforce"]'::jsonb,
            '{"has_authorization_procedures": true, "has_supervision_procedures": true}'::jsonb,
            'Establish authorization procedures for workforce access. Implement supervision and termination procedures.',
            false,
            '2005-04-20'::date,
            '{"section": "164.308(a)(3)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Information Access Management
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(4)',
            'Information Access Management',
            'Implement policies and procedures for authorizing access to ePHI. Implement access establishment and modification procedures.',
            'administrative',
            'critical',
            'mandatory',
            '["patient_record", "system", "user"]'::jsonb,
            '{"has_access_controls": true, "has_access_review_procedures": true}'::jsonb,
            'Implement role-based access controls. Establish procedures for granting and modifying access. Regular access reviews required.',
            false,
            '2005-04-20'::date,
            '{"section": "164.308(a)(4)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Security Awareness and Training
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, check_frequency_days,
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(5)',
            'Security Awareness and Training',
            'Implement a security awareness and training program for all workforce members.',
            'administrative',
            'high',
            'mandatory',
            '["workforce", "employee", "user"]'::jsonb,
            '{"has_training_program": true, "has_annual_training": true}'::jsonb,
            'Develop security awareness training program. Conduct annual training sessions. Document training completion.',
            false,
            365,
            '2005-04-20'::date,
            '{"section": "164.308(a)(5)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Security Incident Procedures
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(6)',
            'Security Incident Procedures',
            'Implement policies and procedures to address security incidents.',
            'administrative',
            'critical',
            'mandatory',
            '["system", "network", "organization"]'::jsonb,
            '{"has_incident_response_plan": true, "has_reporting_procedures": true}'::jsonb,
            'Develop incident response plan. Establish reporting procedures. Define classification criteria.',
            false,
            '2005-04-20'::date,
            '{"section": "164.308(a)(6)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Contingency Plan
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(a)(7)',
            'Contingency Plan',
            'Establish and implement procedures for responding to emergencies or other occurrences that damage systems containing ePHI.',
            'administrative',
            'critical',
            'mandatory',
            '["system", "organization", "backup"]'::jsonb,
            '{"has_backup_plan": true, "has_disaster_recovery": true, "has_testing": true}'::jsonb,
            'Develop data backup plan. Establish disaster recovery procedures. Test contingency plans regularly.',
            false,
            '2005-04-20'::date,
            '{"section": "164.308(a)(7)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Business Associate Contracts
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.308(b)(1)',
            'Business Associate Contracts',
            'Ensure contracts with business associates include appropriate safeguards for ePHI.',
            'administrative',
            'high',
            'mandatory',
            '["vendor", "business_associate", "organization"]'::jsonb,
            '{"has_baa": true, "baa_includes_safeguards": true}'::jsonb,
            'Execute Business Associate Agreements (BAAs) with all business associates. Include required safeguards language.',
            false,
            '2005-04-20'::date,
            '{"section": "164.308(b)(1)", "category": "Administrative Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- ============================================================================
        -- PHYSICAL SAFEGUARDS (45 CFR § 164.310)
        -- ============================================================================

        -- Facility Access Controls
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.310(a)(1)',
            'Facility Access Controls',
            'Implement policies and procedures to limit physical access to facilities containing ePHI.',
            'physical',
            'high',
            'mandatory',
            '["facility", "data_center", "office"]'::jsonb,
            '{"has_access_controls": true, "has_visitor_management": true}'::jsonb,
            'Implement badge access systems. Maintain visitor logs. Control access to server rooms.',
            false,
            '2005-04-20'::date,
            '{"section": "164.310(a)(1)", "category": "Physical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Workstation Use
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.310(c)',
            'Workstation Use',
            'Specify the appropriate functions to be performed at workstations that access ePHI.',
            'physical',
            'medium',
            'mandatory',
            '["workstation", "device"]'::jsonb,
            '{"has_workstation_policies": true}'::jsonb,
            'Establish workstation use policies. Define acceptable use procedures.',
            false,
            '2005-04-20'::date,
            '{"section": "164.310(c)", "category": "Physical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Workstation Security
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.310(c)(2)',
            'Workstation Security',
            'Implement physical safeguards for all workstations that access ePHI to restrict access to authorized users.',
            'physical',
            'high',
            'mandatory',
            '["workstation", "device"]'::jsonb,
            '{"has_locks": true, "has_screensavers": true, "has_password_protection": true}'::jsonb,
            'Implement screen locks with auto-lock. Use privacy filters. Secure workstations when unattended.',
            false,
            '2005-04-20'::date,
            '{"section": "164.310(c)(2)", "category": "Physical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Device and Media Controls
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.310(d)(1)',
            'Device and Media Controls',
            'Implement policies and procedures that govern the receipt and removal of hardware and electronic media containing ePHI.',
            'physical',
            'critical',
            'mandatory',
            '["device", "media", "storage"]'::jsonb,
            '{"has_device_controls": true, "has_media_destruction": true}'::jsonb,
            'Maintain inventory of devices and media. Implement secure disposal procedures. Use encryption for portable devices.',
            false,
            '2005-04-20'::date,
            '{"section": "164.310(d)(1)", "category": "Physical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- ============================================================================
        -- TECHNICAL SAFEGUARDS (45 CFR § 164.312)
        -- ============================================================================

        -- Access Control
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.312(a)(1)',
            'Access Control',
            'Implement technical policies and procedures that allow only authorized persons to access ePHI.',
            'technical',
            'critical',
            'mandatory',
            '["system", "user", "patient_record"]'::jsonb,
            '{"has_authentication": true, "has_authorization": true, "has_access_logs": true}'::jsonb,
            'Implement unique user identification. Use strong authentication. Enforce least-privilege access.',
            true,
            '2005-04-20'::date,
            '{"section": "164.312(a)(1)", "category": "Technical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Audit Controls
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.312(b)',
            'Audit Controls',
            'Implement hardware, software, and/or procedural mechanisms that record and examine activity in information systems containing ePHI.',
            'technical',
            'critical',
            'mandatory',
            '["system", "database", "application"]'::jsonb,
            '{"has_logging": true, "has_log_review": true, "log_retention_days": 6}'::jsonb,
            'Enable comprehensive audit logging. Implement log review procedures. Retain logs per retention policy.',
            true,
            '2005-04-20'::date,
            '{"section": "164.312(b)", "category": "Technical Safeguards", "retention_days": 6}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Integrity
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.312(c)(1)',
            'Integrity',
            'Implement policies and procedures to protect ePHI from improper alteration or destruction.',
            'technical',
            'critical',
            'mandatory',
            '["patient_record", "system"]'::jsonb,
            '{"has_integrity_controls": true, "has_backup": true}'::jsonb,
            'Implement checksums or hashing. Use database constraints. Maintain backup integrity.',
            true,
            '2005-04-20'::date,
            '{"section": "164.312(c)(1)", "category": "Technical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Person or Entity Authentication
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.312(d)',
            'Person or Entity Authentication',
            'Implement procedures to verify that a person or entity seeking access to ePHI is the one claimed.',
            'technical',
            'critical',
            'mandatory',
            '["user", "system", "workstation"]'::jsonb,
            '{"has_strong_passwords": true, "has_mfa": false, "has_session_management": true}'::jsonb,
            'Enforce strong password policies. Implement multi-factor authentication (MFA). Use secure session management.',
            false,
            '2005-04-20'::date,
            '{"section": "164.312(d)", "category": "Technical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Transmission Security
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_security_id,
            'HIPAA-164.312(e)(1)',
            'Transmission Security',
            'Implement technical security measures to guard against unauthorized access to ePHI during transmission.',
            'technical',
            'critical',
            'mandatory',
            '["network", "api", "transmission"]'::jsonb,
            '{"has_encryption": true, "uses_tls": true}'::jsonb,
            'Implement TLS 1.2+ for all ePHI transmissions. Use VPN for remote access. Encrypt email containing ePHI.',
            true,
            '2005-04-20'::date,
            '{"section": "164.312(e)(1)", "category": "Technical Safeguards"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- ============================================================================
        -- PRIVACY RULE RULES
        -- ============================================================================

        -- Minimum Necessary
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_privacy_id,
            'HIPAA-164.502(b)',
            'Minimum Necessary',
            'When using or disclosing PHI, or requesting PHI from another covered entity, make reasonable efforts to limit PHI to the minimum necessary.',
            'privacy',
            'high',
            'mandatory',
            '["patient_record", "disclosure"]'::jsonb,
            '{"has_minimum_necessary_policy": true}'::jsonb,
            'Implement minimum necessary policies. Review data requests. Limit access to only needed information.',
            false,
            '2003-04-14'::date,
            '{"section": "164.502(b)", "category": "Privacy Rule"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Patient Access to PHI
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hipaa_privacy_id,
            'HIPAA-164.524',
            'Individual Access to PHI',
            'Individuals have a right to access and obtain copies of their PHI.',
            'privacy',
            'medium',
            'mandatory',
            '["patient", "patient_record"]'::jsonb,
            '{"has_access_request_procedures": true, "response_time_days": 30}'::jsonb,
            'Establish patient access request procedures. Respond within 30 days. Provide copies in requested format.',
            false,
            '2003-04-14'::date,
            '{"section": "164.524", "category": "Privacy Rule", "response_days": 30}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- ============================================================================
        -- HITECH SPECIFIC RULES
        -- ============================================================================

        -- Business Associate Requirements
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hitech_id,
            'HITECH-BA',
            'Business Associate Direct Liability',
            'Business associates are directly liable for HIPAA violations and subject to civil penalties.',
            'administrative',
            'high',
            'mandatory',
            '["business_associate", "vendor"]'::jsonb,
            '{"has_business_associate_agreements": true}'::jsonb,
            'Execute updated BAAs with all business associates. Ensure BAAs include HITECH language.',
            false,
            '2010-02-17'::date,
            '{"category": "HITECH Enhancements"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Breach Notification
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            hitech_id,
            'HITECH-BREACH',
            'Breach Notification',
            'Notify affected individuals, HHS, and media (if >500 affected) of any breach of unsecured PHI within specified timeframes.',
            'administrative',
            'critical',
            'mandatory',
            '["breach", "incident"]'::jsonb,
            '{"has_notification_procedures": true, "has_risk_assessment": true}'::jsonb,
            'Develop breach notification procedures. Document breach risk assessments. Maintain breach log.',
            false,
            '2009-09-23'::date,
            '{"individual_notification_days": 60, "hhs_notification_days": 60, "media_threshold": 500}'::jsonb
        ) ON CONFLICT DO NOTHING;

        RAISE NOTICE 'HIPAA compliance rules seeded successfully';
    ELSE
        RAISE NOTICE 'HIPAA frameworks not found. Run global_compliance_frameworks.sql first.';
    END IF;
END $$;

COMMENT ON TABLE compliance_rules IS 'HIPAA compliance rules based on CFR 45 Part 164';


