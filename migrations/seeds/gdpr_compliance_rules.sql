-- GDPR Compliance Rules Seed Data
-- Based on GDPR Articles for healthcare data processing
-- Regulation (EU) 2016/679

DO $$
DECLARE
    gdpr_id UUID;
BEGIN
    SELECT id INTO gdpr_id FROM compliance_frameworks WHERE code = 'GDPR' LIMIT 1;

    IF gdpr_id IS NOT NULL THEN

        -- Article 5: Principles of Processing
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART5',
            'Principles Relating to Processing',
            'Personal data must be processed lawfully, fairly, transparently; collected for specified purposes; adequate and limited to what is necessary; accurate; kept in identifiable form no longer than necessary; and processed securely.',
            'data_protection',
            'critical',
            'mandatory',
            '["personal_data", "health_data"]'::jsonb,
            '{"has_lawful_basis": true, "has_transparency": true}'::jsonb,
            'Document lawful basis for processing. Ensure fairness and transparency. Implement purpose limitation.',
            false,
            '2018-05-25'::date,
            '{"article": "5", "regulation": "EU 2016/679"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 6: Lawfulness of Processing
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART6',
            'Lawfulness of Processing',
            'Processing is lawful only if based on consent, contract, legal obligation, vital interests, public task, or legitimate interests.',
            'legal_basis',
            'critical',
            'mandatory',
            '["processing", "data_collection"]'::jsonb,
            '{"has_legal_basis": true, "basis_documented": true}'::jsonb,
            'Identify and document lawful basis for all processing activities. For health data, ensure proper exemption conditions.',
            false,
            '2018-05-25'::date,
            '{"article": "6", "regulation": "EU 2016/679"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 9: Special Categories of Data (Health Data)
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART9',
            'Processing of Special Categories of Data',
            'Processing of health data is prohibited unless exempt by Article 9(2), e.g., explicit consent, employment law, medical diagnosis, public health, or legitimate activities of foundations.',
            'data_protection',
            'critical',
            'mandatory',
            '["health_data", "sensitive_data", "biometric_data"]'::jsonb,
            '{"has_explicit_consent": true, "has_proper_exemption": true}'::jsonb,
            'Obtain explicit consent for health data processing or ensure proper exemption applies. Document basis.',
            false,
            '2018-05-25'::date,
            '{"article": "9", "regulation": "EU 2016/679", "special_category": true}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 15: Right of Access
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART15',
            'Right of Access',
            'Data subjects have the right to access their personal data, including purposes, categories, recipients, retention, rights to rectification, erasure, restriction, objection, right to lodge complaint, and information on automated decision-making.',
            'data_subject_rights',
            'high',
            'mandatory',
            '["personal_data", "data_subject"]'::jsonb,
            '{"has_access_procedures": true, "response_time_days": 30}'::jsonb,
            'Establish data subject access request procedures. Respond within one month. Provide comprehensive data copy.',
            false,
            '2018-05-25'::date,
            '{"article": "15", "response_days": 30}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 17: Right to Erasure (Right to be Forgotten)
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART17',
            'Right to Erasure',
            'Data subjects have the right to erasure of their personal data where data is no longer necessary, consent withdrawn, or processing unlawful.',
            'data_subject_rights',
            'high',
            'mandatory',
            '["personal_data", "data_subject"]'::jsonb,
            '{"has_erasure_procedures": true, "has_verification": true}'::jsonb,
            'Establish erasure request procedures. Verify identity. Implement secure deletion. Notify third parties.',
            false,
            '2018-05-25'::date,
            '{"article": "17"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 25: Data Protection by Design and by Default
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART25',
            'Data Protection by Design and Default',
            'Implement appropriate technical and organizational measures to ensure data protection principles are effectively implemented, including data minimization, pseudonymization, encryption, and access controls.',
            'technical',
            'critical',
            'mandatory',
            '["system", "application", "database"]'::jsonb,
            '{"has_privacy_by_design": true, "has_minimization": true, "has_encryption": true}'::jsonb,
            'Conduct privacy impact assessments (PIA). Implement data minimization. Use pseudonymization and encryption.',
            false,
            '2018-05-25'::date,
            '{"article": "25", "pseudonymization": true, "encryption": true}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 32: Security of Processing
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART32',
            'Security of Processing',
            'Implement appropriate technical and organizational measures including pseudonymization, encryption, confidentiality, availability, resilience, and regular testing to ensure security of processing.',
            'technical',
            'critical',
            'mandatory',
            '["system", "network", "database"]'::jsonb,
            '{"has_encryption": true, "has_access_controls": true, "has_testing": true}'::jsonb,
            'Implement encryption at rest and in transit. Establish access controls. Conduct regular security testing.',
            true,
            '2018-05-25'::date,
            '{"article": "32", "encryption_at_rest": true, "encryption_in_transit": true}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 33: Breach Notification
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART33',
            'Personal Data Breach Notification to Supervisory Authority',
            'Notify supervisory authority of personal data breach within 72 hours unless unlikely to result in risk to rights and freedoms.',
            'incident',
            'critical',
            'mandatory',
            '["breach", "incident"]'::jsonb,
            '{"has_notification_procedures": true, "notification_hours": 72}'::jsonb,
            'Establish breach detection procedures. Notify supervisory authority within 72 hours. Document all breaches.',
            false,
            '2018-05-25'::date,
            '{"article": "33", "notification_hours": 72}'::jsonb
        ) ON CONFLICT DO NOTHING;

        -- Article 35: Data Protection Impact Assessment
        INSERT INTO compliance_rules (
            organization_id, framework_id, rule_code, title, description, 
            category, severity, rule_type, applies_to_entity_types,
            validation_logic, remediation_steps, is_automated, 
            effective_date, metadata
        ) VALUES (
            '00000000-0000-0000-0000-000000000000',
            gdpr_id,
            'GDPR-ART35',
            'Data Protection Impact Assessment',
            'Conduct DPIA for processing likely to result in high risk to rights and freedoms, including systematic monitoring, special data categories, or automated decision-making.',
            'governance',
            'high',
            'mandatory',
            '["project", "processing", "system"]'::jsonb,
            '{"has_dpia_procedures": true, "dpia_conducted": true}'::jsonb,
            'Conduct DPIAs for high-risk processing. Document assessments. Consult supervisory authority if needed.',
            false,
            '2018-05-25'::date,
            '{"article": "35"}'::jsonb
        ) ON CONFLICT DO NOTHING;

        RAISE NOTICE 'GDPR compliance rules seeded successfully';
    ELSE
        RAISE NOTICE 'GDPR framework not found. Run global_compliance_frameworks.sql first.';
    END IF;
END $$;

