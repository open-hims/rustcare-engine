// Email mailbox verification without sending emails
use crate::error::{EmailError, EmailResult};
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::*;
use tracing::{info, warn};

/// Verify if an email mailbox exists by checking MX records
/// This validates that the domain can receive emails
pub async fn verify_mailbox_exists(email: &str) -> EmailResult<bool> {
    info!(email = %email, "Starting mailbox verification");

    // Parse email domain
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(EmailError::SendFailed(format!("Invalid email format: {}", email)));
    }

    let domain = parts[1];
    info!(domain = %domain, "Looking up MX records");

    // Create resolver
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    // Look up MX records
    let mx_lookup = resolver.mx_lookup(domain.to_string()).await
        .map_err(|e| EmailError::SendFailed(format!("MX lookup failed: {}", e)))?;

    let mut mx_records: Vec<(u16, String)> = mx_lookup
        .iter()
        .map(|mx| (mx.preference(), mx.exchange().to_utf8()))
        .collect();

    mx_records.sort_by_key(|r| r.0);

    if mx_records.is_empty() {
        warn!(domain = %domain, "No MX records found");
        return Ok(false);
    }

    let mx_host = &mx_records[0].1;
    info!(mx_host = %mx_host, "Found MX server");

    // If we have MX records, the domain can receive emails
    // For true mailbox existence, we would need SMTP RCPT TO validation
    // but that's not always reliable due to anti-spam measures
    info!(email = %email, domain = %domain, "Email domain has valid MX records");
    Ok(true)
}

/// Verify if an email domain has valid MX records
/// This is a lightweight check that validates the domain setup
pub async fn verify_domain_mx(email: &str) -> EmailResult<bool> {
    verify_mailbox_exists(email).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires actual network access
    async fn test_verify_mailbox_exists() {
        // Test with a real email address
        let result = verify_mailbox_exists("test@gmail.com").await;
        println!("Result: {:?}", result);
    }
}
