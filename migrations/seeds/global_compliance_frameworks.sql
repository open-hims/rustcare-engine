-- Seed Global Compliance Frameworks
-- This file seeds the database with major healthcare compliance frameworks and regulations worldwide
-- Run this after the create_compliance_framework migration

-- Helper function to get UUID for organizations (using nil for global frameworks)
-- In production, these will be organization-specific

-- ============================================================================
-- US HEALTHCARE REGULATIONS
-- ============================================================================

-- HIPAA (Health Insurance Portability and Accountability Act)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, parent_framework_id, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000', -- Global
    'Health Insurance Portability and Accountability Act',
    'HIPAA',
    '1996',
    'Federal law requiring healthcare providers to protect patient health information (PHI). Includes Privacy Rule, Security Rule, and Breach Notification Rule.',
    'US Department of Health and Human Services (HHS)',
    'United States',
    '1996-08-21',
    'active',
    NULL,
    '{"website": "https://www.hhs.gov/hipaa", "updated": "2013", "penalties": {"max_per_violation": 50000, "annual_limit": 1500000}}'::jsonb
) ON CONFLICT DO NOTHING;

-- HIPAA Privacy Rule
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, parent_framework_id, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'HIPAA Privacy Rule',
    'HIPAA-PRIVACY',
    '2003',
    'Establishes national standards for the protection of individually identifiable health information.',
    'HHS Office for Civil Rights',
    'United States',
    '2003-04-14',
    'active',
    (SELECT id FROM compliance_frameworks WHERE code = 'HIPAA'),
    '{"45_cfr": "164.500-534"}'::jsonb
) ON CONFLICT DO NOTHING;

-- HIPAA Security Rule
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, parent_framework_id, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'HIPAA Security Rule',
    'HIPAA-SECURITY',
    '2005',
    'Sets national standards for protecting electronic PHI. Requires administrative, physical, and technical safeguards.',
    'HHS Office for Civil Rights',
    'United States',
    '2005-04-20',
    'active',
    (SELECT id FROM compliance_frameworks WHERE code = 'HIPAA'),
    '{"45_cfr": "164.302-318"}'::jsonb
) ON CONFLICT DO NOTHING;

-- HITECH Act
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Health Information Technology for Economic and Clinical Health Act',
    'HITECH',
    '2009',
    'Enhances HIPAA enforcement, extends liability to business associates, strengthens breach notification requirements.',
    'HHS',
    'United States',
    '2009-02-17',
    'active',
    '{"website": "https://www.healthit.gov/topic/laws-regulation-and-policy/health-it-legislation"}'::jsonb
) ON CONFLICT DO NOTHING;

-- 21 CFR Part 11 (FDA Electronic Records)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'FDA 21 CFR Part 11',
    '21CFR11',
    '1997',
    'FDA regulation for electronic records and electronic signatures in FDA-regulated industries, including pharmaceuticals and medical devices.',
    'US Food and Drug Administration (FDA)',
    'United States',
    '1997-08-20',
    'active',
    '{"website": "https://www.fda.gov/regulatory-information/search-fda-guidance-documents/part-11-electronic-records-electronic-signatures-scope-and-application"}'::jsonb
) ON CONFLICT DO NOTHING;

-- SOX (Sarbanes-Oxley Act)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Sarbanes-Oxley Act',
    'SOX',
    '2002',
    'Financial and accounting transparency regulations. Applies to publicly traded healthcare companies.',
    'SEC',
    'United States',
    '2002-07-30',
    'active',
    '{"website": "https://www.sec.gov/spotlight/sarbanes-oxley.htm"}'::jsonb
) ON CONFLICT DO NOTHING;

-- CCPA (California Consumer Privacy Act)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'California Consumer Privacy Act',
    'CCPA',
    '2020',
    'California state law granting consumers control over personal information collected by businesses.',
    'California Attorney General',
    'California, United States',
    '2020-01-01',
    'active',
    '{"website": "https://oag.ca.gov/privacy/ccpa"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- EUROPEAN UNION REGULATIONS
-- ============================================================================

-- GDPR (General Data Protection Regulation)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'General Data Protection Regulation',
    'GDPR',
    '2018',
    'EU regulation protecting personal data and privacy. Applies to any organization processing EU residents data.',
    'European Commission',
    'European Union',
    '2018-05-25',
    'active',
    '{"website": "https://gdpr.eu", "regulation": "2016/679", "penalties": {"max_percent": 4, "annual_limit": 20000000}}'::jsonb
) ON CONFLICT DO NOTHING;

-- MDR (Medical Device Regulation EU)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Medical Device Regulation',
    'MDR-EU',
    '2021',
    'EU regulation governing medical devices, including software as a medical device.',
    'European Commission',
    'European Union',
    '2021-05-26',
    'active',
    '{"website": "https://ec.europa.eu/health/md_sector/overview", "regulation": "2017/745"}'::jsonb
) ON CONFLICT DO NOTHING;

-- IVDR (In Vitro Diagnostic Regulation EU)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'In Vitro Diagnostic Medical Devices Regulation',
    'IVDR-EU',
    '2022',
    'EU regulation for in vitro diagnostic medical devices including laboratory information systems.',
    'European Commission',
    'European Union',
    '2022-05-26',
    'active',
    '{"regulation": "2017/746"}'::jsonb
) ON CONFLICT DO NOTHING;

-- UK GDPR
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'UK General Data Protection Regulation',
    'UK-GDPR',
    '2021',
    'UK data protection law post-Brexit, based on EU GDPR.',
    'Information Commissioner Office (ICO)',
    'United Kingdom',
    '2021-01-01',
    'active',
    '{"website": "https://ico.org.uk"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- CANADA REGULATIONS
-- ============================================================================

-- PIPEDA
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Personal Information Protection and Electronic Documents Act',
    'PIPEDA',
    '2000',
    'Canadian federal law governing how private sector organizations collect, use and disclose personal information.',
    'Office of the Privacy Commissioner of Canada',
    'Canada',
    '2000-04-13',
    'active',
    '{"website": "https://www.priv.gc.ca/en/privacy-topics/privacy-laws-in-canada"}'::jsonb
) ON CONFLICT DO NOTHING;

-- PHIPA (Ontario)
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Personal Health Information Protection Act',
    'PHIPA',
    '2004',
    'Ontario provincial law protecting personal health information.',
    'Information and Privacy Commissioner of Ontario',
    'Ontario, Canada',
    '2004-11-01',
    'active',
    '{"website": "https://www.ipc.on.ca"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- AUSTRALIA REGULATIONS
-- ============================================================================

-- Privacy Act
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Privacy Act 1988',
    'AU-PRIVACY',
    '1988',
    'Australian privacy legislation including Australian Privacy Principles (APPs).',
    'Office of the Australian Information Commissioner (OAIC)',
    'Australia',
    '1988-12-14',
    'active',
    '{"website": "https://www.oaic.gov.au"}'::jsonb
) ON CONFLICT DO NOTHING;

-- My Health Records Act
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'My Health Records Act',
    'MHRA',
    '2012',
    'Australian legislation governing the My Health Record system.',
    'Australian Digital Health Agency',
    'Australia',
    '2012-07-01',
    'active',
    '{"website": "https://www.myhealthrecord.gov.au"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- BRAZIL REGULATIONS
-- ============================================================================

-- LGPD
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Lei Geral de Proteção de Dados',
    'LGPD',
    '2020',
    'Brazilian General Data Protection Law, similar to GDPR.',
    'Autoridade Nacional de Proteção de Dados (ANPD)',
    'Brazil',
    '2020-08-16',
    'active',
    '{"website": "https://www.gov.br/anpd", "law_number": "13709/2018"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- INDIA REGULATIONS
-- ============================================================================

-- DPDP Act
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Digital Personal Data Protection Act',
    'DPDP-2023',
    '2023',
    'Indian comprehensive data protection law for digital personal data.',
    'Data Protection Board of India',
    'India',
    '2023-08-11',
    'active',
    '{"website": "https://www.meity.gov.in"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- ISO STANDARDS
-- ============================================================================

-- ISO 27001
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'ISO/IEC 27001 - Information Security Management',
    'ISO27001',
    '2022',
    'International standard for information security management systems.',
    'International Organization for Standardization',
    'International',
    '2022-10-25',
    'active',
    '{"website": "https://www.iso.org/standard/27001"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ISO 27018
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, parent_framework_id, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'ISO/IEC 27018 - Cloud Privacy',
    'ISO27018',
    '2019',
    'Code of practice for protection of personally identifiable information in public cloud services.',
    'ISO',
    'International',
    '2019-07-01',
    'active',
    (SELECT id FROM compliance_frameworks WHERE code = 'ISO27001'),
    '{"cloud_specific": true}'::jsonb
) ON CONFLICT DO NOTHING;

-- ISO 13485
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'ISO 13485 - Medical Devices Quality Management',
    'ISO13485',
    '2016',
    'Quality management system standard for medical device manufacturing and distribution.',
    'ISO',
    'International',
    '2016-03-01',
    'active',
    '{"website": "https://www.iso.org/standard/59752.html"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ISO 27799
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'ISO 27799 - Health Informatics Security',
    'ISO27799',
    '2016',
    'Security management in health informatics using ISO/IEC 27002.',
    'ISO',
    'International',
    '2016-06-01',
    'active',
    '{"related_to": "ISO27001"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- NIST FRAMEWORKS
-- ============================================================================

-- NIST CSF
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'NIST Cybersecurity Framework',
    'NIST-CSF',
    '2.0',
    'Voluntary framework consisting of standards, guidelines, and best practices for managing cybersecurity risk.',
    'National Institute of Standards and Technology',
    'United States (International Application)',
    '2024-02-26',
    'active',
    '{"website": "https://www.nist.gov/cyberframework", "version": "2.0"}'::jsonb
) ON CONFLICT DO NOTHING;

-- NIST 800-53
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'NIST SP 800-53 - Security and Privacy Controls',
    'NIST800-53',
    'R5',
    'Catalog of security and privacy controls for federal information systems.',
    'NIST',
    'United States',
    '2020-09-23',
    'active',
    '{"website": "https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- HITRUST
-- ============================================================================

-- HITRUST CSF
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'HITRUST Common Security Framework',
    'HITRUST-CSF',
    '11.0',
    'Comprehensive, certifiable framework for healthcare data protection and regulatory compliance.',
    'HITRUST Alliance',
    'United States (International Application)',
    '2024-01-01',
    'active',
    '{"website": "https://hitrustalliance.net", "integrates": ["HIPAA", "NIST-CSF", "ISO27001"]}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- SOC 2
-- ============================================================================

-- SOC 2 Type II
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'SOC 2 Type II',
    'SOC2',
    '2017',
    'Trust Services Criteria for security, availability, processing integrity, confidentiality, and privacy.',
    'AICPA',
    'United States (International Application)',
    '2017-05-01',
    'active',
    '{"website": "https://www.aicpa.org"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- PCI DSS (For Healthcare Payment Processing)
-- ============================================================================

-- PCI DSS
INSERT INTO compliance_frameworks (
    organization_id, name, code, version, description, authority, jurisdiction, 
    effective_date, status, metadata
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Payment Card Industry Data Security Standard',
    'PCI-DSS',
    '4.0',
    'Security standard for organizations that handle payment card information.',
    'PCI Security Standards Council',
    'International',
    '2024-03-31',
    'active',
    '{"website": "https://www.pcisecuritystandards.org"}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE compliance_frameworks IS 'Includes major global healthcare compliance frameworks and regulations';
COMMENT ON COLUMN compliance_frameworks.metadata IS 'JSONB field containing framework-specific metadata, websites, related regulations, penalties';

-- Note: This seed data uses organization_id '00000000-0000-0000-0000-000000000000' for global frameworks
-- Individual organizations should clone or reference these frameworks for their specific compliance needs


