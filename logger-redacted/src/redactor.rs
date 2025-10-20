use regex::Regex;
use lazy_static::lazy_static;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    static ref PHONE_REGEX: Regex = Regex::new(r"\b(?:\+1[-.\s]?)?\(?([0-9]{3})\)?[-.\s]?([0-9]{3})[-.\s]?([0-9]{4})\b").unwrap();
    static ref SSN_REGEX: Regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    static ref CREDIT_CARD_REGEX: Regex = Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap();
    static ref IP_REGEX: Regex = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap();
}

/// PII redaction configuration
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    pub redact_emails: bool,
    pub redact_phones: bool,
    pub redact_ssn: bool,
    pub redact_credit_cards: bool,
    pub redact_ip_addresses: bool,
    pub hash_for_correlation: bool,
    pub custom_patterns: Vec<(Regex, String)>,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            redact_emails: true,
            redact_phones: true,
            redact_ssn: true,
            redact_credit_cards: true,
            redact_ip_addresses: true,
            hash_for_correlation: true,
            custom_patterns: Vec::new(),
        }
    }
}

/// PII redactor for log messages
pub struct PiiRedactor {
    config: RedactionConfig,
}

impl PiiRedactor {
    pub fn new(config: RedactionConfig) -> Self {
        Self { config }
    }
    
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        
        if self.config.redact_emails {
            result = self.redact_emails(&result);
        }
        
        if self.config.redact_phones {
            result = self.redact_phones(&result);
        }
        
        if self.config.redact_ssn {
            result = self.redact_ssn(&result);
        }
        
        if self.config.redact_credit_cards {
            result = self.redact_credit_cards(&result);
        }
        
        if self.config.redact_ip_addresses {
            result = self.redact_ip_addresses(&result);
        }
        
        for (pattern, replacement) in &self.config.custom_patterns {
            result = pattern.replace_all(&result, replacement).to_string();
        }
        
        result
    }
    
    fn redact_emails(&self, text: &str) -> String {
        EMAIL_REGEX.replace_all(text, |caps: &regex::Captures| {
            let email = &caps[0];
            if self.config.hash_for_correlation {
                format!("EMAIL[{}]", self.hash_value(email))
            } else {
                let parts: Vec<&str> = email.split('@').collect();
                if parts.len() == 2 {
                    format!("{}***@{}***", &parts[0][..1.min(parts[0].len())], &parts[1][..1.min(parts[1].len())])
                } else {
                    "***@***.com".to_string()
                }
            }
        }).to_string()
    }
    
    fn redact_phones(&self, text: &str) -> String {
        PHONE_REGEX.replace_all(text, |caps: &regex::Captures| {
            if self.config.hash_for_correlation {
                format!("PHONE[{}]", self.hash_value(&caps[0]))
            } else {
                "(***) ***-****".to_string()
            }
        }).to_string()
    }
    
    fn redact_ssn(&self, text: &str) -> String {
        SSN_REGEX.replace_all(text, |caps: &regex::Captures| {
            if self.config.hash_for_correlation {
                format!("SSN[{}]", self.hash_value(&caps[0]))
            } else {
                "***-**-****".to_string()
            }
        }).to_string()
    }
    
    fn redact_credit_cards(&self, text: &str) -> String {
        CREDIT_CARD_REGEX.replace_all(text, |caps: &regex::Captures| {
            if self.config.hash_for_correlation {
                format!("CC[{}]", self.hash_value(&caps[0]))
            } else {
                "****-****-****-****".to_string()
            }
        }).to_string()
    }
    
    fn redact_ip_addresses(&self, text: &str) -> String {
        IP_REGEX.replace_all(text, |caps: &regex::Captures| {
            if self.config.hash_for_correlation {
                format!("IP[{}]", self.hash_value(&caps[0]))
            } else {
                let ip = &caps[0];
                let parts: Vec<&str> = ip.split('.').collect();
                if parts.len() == 4 {
                    format!("{}.***.***.{}", parts[0], parts[3])
                } else {
                    "***.***.***.***".to_string()
                }
            }
        }).to_string()
    }
    
    fn hash_value(&self, value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        let result = hasher.finalize();
        general_purpose::STANDARD.encode(&result[..8]) // Use first 8 bytes for shorter hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_redaction() {
        let redactor = PiiRedactor::new(RedactionConfig {
            hash_for_correlation: false,
            ..Default::default()
        });
        
        let text = "User john.doe@example.com logged in";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("j***@e***"));
    }
    
    #[test]
    fn test_phone_redaction() {
        let redactor = PiiRedactor::new(RedactionConfig {
            hash_for_correlation: false,
            ..Default::default()
        });
        
        let text = "Call me at (555) 123-4567";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("(***) ***-****"));
    }
}