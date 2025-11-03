// Email mailbox verification without sending emails
use crate::error::{EmailError, EmailResult};
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::*;
use tracing::{info, warn, debug};
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::time::Duration;

/// Verify if an email mailbox exists by checking MX records
/// This validates that the domain can receive emails
pub async fn verify_mailbox_exists(email: &str) -> EmailResult<bool> {
    verify_mailbox_exists_smtp(email).await
}

/// Verify mailbox existence using SMTP RCPT TO command
/// 
/// This connects to the MX server and verifies if the email address exists by:
/// 1. Looking up MX records for the domain
/// 2. Connecting to the MX server (port 25)
/// 3. Sending HELO, MAIL FROM, and RCPT TO commands
/// 4. Checking if RCPT TO returns 250 (mailbox exists)
/// 
/// **Note**: Many email providers (Gmail, Outlook, etc.) block SMTP RCPT TO verification
/// for spam prevention. This method works best with self-hosted email servers or
/// providers that allow this verification method.
/// 
/// Returns true if the server returns 250 (success), false otherwise
pub async fn verify_mailbox_exists_smtp(email: &str) -> EmailResult<bool> {
    info!(email = %email, "Starting SMTP mailbox verification");

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

    // Get local hostname for HELO
    let hostname = hostname::get()
        .map_err(|e| EmailError::SendFailed(format!("Failed to get hostname: {}", e)))?
        .to_string_lossy()
        .to_string();

    // Try each MX record in order of preference
    for (preference, mx_host) in &mx_records {
        info!(mx_host = %mx_host, preference = %preference, "Attempting SMTP verification");

        match verify_with_mx_server(email, mx_host, &hostname).await {
            Ok(true) => {
                info!(email = %email, mx_host = %mx_host, "Mailbox verified successfully");
                return Ok(true);
            }
            Ok(false) => {
                warn!(email = %email, mx_host = %mx_host, "Mailbox verification failed");
                // Continue to next MX server
            }
            Err(e) => {
                warn!(mx_host = %mx_host, error = %e, "Error connecting to MX server, trying next");
                // Continue to next MX server
            }
        }
    }

    warn!(email = %email, "All MX servers failed or rejected the address");
    Ok(false)
}

/// Verify mailbox with a specific MX server using SMTP RCPT TO
async fn verify_with_mx_server(
    email: &str,
    mx_host: &str,
    local_hostname: &str,
) -> EmailResult<bool> {
    // Connect to MX server on port 25
    let addr = format!("{}:25", mx_host);
    debug!(mx_host = %mx_host, addr = %addr, "Connecting to MX server");

    let stream = tokio::time::timeout(
        Duration::from_secs(10),
        TcpStream::connect(&addr)
    ).await
        .map_err(|_| EmailError::SendFailed(format!("Connection timeout to {}", mx_host)))?
        .map_err(|e| EmailError::SendFailed(format!("Failed to connect to {}: {}", mx_host, e)))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let mut line = String::new();
    reader.read_line(&mut line).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to read SMTP greeting: {}", e)))?;
    
    debug!(response = %line.trim(), "SMTP greeting received");
    
    // Check if greeting is OK (usually 220)
    if !line.starts_with("220") {
        warn!(response = %line.trim(), "Unexpected SMTP greeting");
    }

    // HELO command
    let helo_cmd = format!("HELO {}\r\n", local_hostname);
    debug!(command = %helo_cmd.trim(), "Sending HELO");
    writer.write_all(helo_cmd.as_bytes()).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to send HELO: {}", e)))?;

    line.clear();
    reader.read_line(&mut line).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to read HELO response: {}", e)))?;
    
    debug!(response = %line.trim(), "HELO response received");
    
    if !line.starts_with("250") {
        warn!(response = %line.trim(), "HELO command failed");
        return Ok(false);
    }

    // MAIL FROM command
    // Use a test email address - in production this could be configurable
    let mail_from = std::env::var("EMAIL_VERIFICATION_FROM")
        .unwrap_or_else(|_| "noreply@rustcare.dev".to_string());
    let mail_cmd = format!("MAIL FROM:<{}>\r\n", mail_from);
    debug!(command = %mail_cmd.trim(), "Sending MAIL FROM");
    writer.write_all(mail_cmd.as_bytes()).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to send MAIL FROM: {}", e)))?;

    line.clear();
    reader.read_line(&mut line).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to read MAIL FROM response: {}", e)))?;
    
    debug!(response = %line.trim(), "MAIL FROM response received");
    
    if !line.starts_with("250") {
        warn!(response = %line.trim(), "MAIL FROM command failed");
        return Ok(false);
    }

    // RCPT TO command - this is where we check if the mailbox exists
    let rcpt_cmd = format!("RCPT TO:<{}>\r\n", email);
    debug!(command = %rcpt_cmd.trim(), "Sending RCPT TO");
    writer.write_all(rcpt_cmd.as_bytes()).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to send RCPT TO: {}", e)))?;

    line.clear();
    reader.read_line(&mut line).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to read RCPT TO response: {}", e)))?;
    
    debug!(response = %line.trim(), "RCPT TO response received");
    
    // Check if response code is 250 (success)
    let exists = line.starts_with("250");
    
    // QUIT command
    let quit_cmd = "QUIT\r\n";
    debug!(command = %quit_cmd.trim(), "Sending QUIT");
    writer.write_all(quit_cmd.as_bytes()).await
        .map_err(|e| EmailError::SendFailed(format!("Failed to send QUIT: {}", e)))?;

    // Read QUIT response (optional)
    line.clear();
    let _ = reader.read_line(&mut line).await; // Ignore errors on QUIT response

    Ok(exists)
}

/// Verify if an email domain has valid MX records
/// This is a lightweight check that validates the domain setup
pub async fn verify_domain_mx(email: &str) -> EmailResult<bool> {
    info!(email = %email, "Verifying domain MX records");

    // Parse email domain
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(EmailError::SendFailed(format!("Invalid email format: {}", email)));
    }

    let domain = parts[1];
    
    // Create resolver
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    // Look up MX records
    let mx_lookup = resolver.mx_lookup(domain.to_string()).await
        .map_err(|e| EmailError::SendFailed(format!("MX lookup failed: {}", e)))?;

    let mx_records: Vec<(u16, String)> = mx_lookup
        .iter()
        .map(|mx| (mx.preference(), mx.exchange().to_utf8()))
        .collect();

    if mx_records.is_empty() {
        warn!(domain = %domain, "No MX records found");
        return Ok(false);
    }

    info!(domain = %domain, count = mx_records.len(), "Domain has valid MX records");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires actual network access
    async fn test_verify_mailbox_exists_smtp() {
        // Test with a real email address using SMTP RCPT TO
        let result = verify_mailbox_exists_smtp("amalandomnic@gmail.com").await;
        println!("SMTP verification result: {:?}", result);
    }

    #[tokio::test]
    #[ignore] // Requires actual network access
    async fn test_verify_domain_mx() {
        // Test domain MX record lookup
        let result = verify_domain_mx("test@gmail.com").await;
        println!("MX record verification result: {:?}", result);
    }
}
