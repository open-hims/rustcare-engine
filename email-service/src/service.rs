// Email service implementation with Stalwart mail-send
use crate::error::{EmailError, EmailResult};
use mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Email service configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
    pub smtp_tls_enabled: bool,
    pub email_enabled: bool,
}

impl EmailConfig {
    /// Load email configuration from environment variables
    pub fn from_env() -> EmailResult<Self> {
        Ok(Self {
            smtp_host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            smtp_username: std::env::var("SMTP_USERNAME")
                .unwrap_or_else(|_| "admin@rustcare.local".to_string()),
            smtp_password: std::env::var("SMTP_PASSWORD")
                .unwrap_or_else(|_| "changeme".to_string()),
            smtp_from_email: std::env::var("SMTP_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@rustcare.local".to_string()),
            smtp_from_name: std::env::var("SMTP_FROM_NAME")
                .unwrap_or_else(|_| "RustCare Engine".to_string()),
            smtp_tls_enabled: std::env::var("SMTP_TLS_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            email_enabled: std::env::var("EMAIL_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
        })
    }
}

/// Email service for sending transactional emails via Stalwart
pub struct EmailService {
    config: EmailConfig,
}

impl EmailService {
    /// Create a new email service
    pub fn new(config: EmailConfig) -> EmailResult<Self> {
        if !config.email_enabled {
            info!("Email service disabled by configuration");
        }
        Ok(Self { config })
    }

    /// Send a plain text email
    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> EmailResult<String> {
        if !self.config.email_enabled {
            debug!("Email disabled, skipping send to: {}", to);
            return Ok(format!("disabled-{}", Uuid::new_v4()));
        }

        let message = MessageBuilder::new()
            .from((
                self.config.smtp_from_name.as_str(),
                self.config.smtp_from_email.as_str(),
            ))
            .to(to)
            .subject(subject)
            .text_body(body);

        self.send_message(message).await
    }

    /// Send an HTML email
    pub async fn send_html_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> EmailResult<String> {
        if !self.config.email_enabled {
            debug!("Email disabled, skipping send to: {}", to);
            return Ok(format!("disabled-{}", Uuid::new_v4()));
        }

        let message = MessageBuilder::new()
            .from((
                self.config.smtp_from_name.as_str(),
                self.config.smtp_from_email.as_str(),
            ))
            .to(to)
            .subject(subject)
            .html_body(html_body);

        self.send_message(message).await
    }

    /// Send organization welcome email
    pub async fn send_organization_welcome(
        &self,
        to_email: &str,
        org_name: &str,
        org_slug: &str,
    ) -> EmailResult<String> {
        let subject = format!("Welcome to RustCare - {} is ready!", org_name);
        let body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Welcome to RustCare</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); padding: 30px; border-radius: 10px 10px 0 0;">
        <h1 style="color: white; margin: 0; font-size: 28px;">Welcome to RustCare!</h1>
    </div>
    
    <div style="background: #f9f9f9; padding: 30px; border-radius: 0 0 10px 10px;">
        <h2 style="color: #667eea; margin-top: 0;">Your Organization is Ready</h2>
        
        <p>Hello,</p>
        
        <p>Congratulations! Your organization <strong>{}</strong> has been successfully created on RustCare.</p>
        
        <div style="background: white; padding: 20px; border-left: 4px solid #667eea; margin: 20px 0;">
            <h3 style="margin-top: 0; color: #667eea;">Organization Details</h3>
            <p style="margin: 10px 0;"><strong>Name:</strong> {}</p>
            <p style="margin: 10px 0;"><strong>Slug:</strong> {}</p>
            <p style="margin: 10px 0;"><strong>Dashboard:</strong> <a href="http://localhost:3000/org/{}" style="color: #667eea;">Access Dashboard</a></p>
        </div>
        
        <h3 style="color: #667eea;">Next Steps:</h3>
        <ul style="line-height: 2;">
            <li>Invite team members to your organization</li>
            <li>Configure organization settings and permissions</li>
            <li>Set up custom email domain (optional)</li>
            <li>Start managing your healthcare workflows</li>
        </ul>
        
        <p style="margin-top: 30px;">If you have any questions, please don't hesitate to reach out to our support team.</p>
        
        <p>Best regards,<br>The RustCare Team</p>
        
        <hr style="border: none; border-top: 1px solid #ddd; margin: 30px 0;">
        
        <p style="font-size: 12px; color: #666; text-align: center;">
            This is an automated message from RustCare Engine<br>
            © 2024 RustCare. All rights reserved.
        </p>
    </div>
</body>
</html>
            "#,
            org_name, org_name, org_slug, org_slug
        );

        info!(
            organization = org_name,
            email = to_email,
            "Sending organization welcome email"
        );

        self.send_html_email(to_email, &subject, &body).await
    }

    /// Send email verification with DNS records
    pub async fn send_email_domain_verification(
        &self,
        to_email: &str,
        org_name: &str,
        domain: &str,
        verification_token: &str,
    ) -> EmailResult<String> {
        let subject = format!("Verify Email Domain for {}", org_name);
        let body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Email Domain Verification</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); padding: 30px; border-radius: 10px 10px 0 0;">
        <h1 style="color: white; margin: 0; font-size: 28px;">Email Domain Verification</h1>
    </div>
    
    <div style="background: #f9f9f9; padding: 30px; border-radius: 0 0 10px 10px;">
        <h2 style="color: #667eea; margin-top: 0;">Set Up Custom Email Domain</h2>
        
        <p>Hello,</p>
        
        <p>To enable custom email sending from <strong>{}</strong>, please add the following DNS records:</p>
        
        <div style="background: white; padding: 20px; border-left: 4px solid #667eea; margin: 20px 0;">
            <h3 style="margin-top: 0; color: #667eea;">TXT Record for Verification</h3>
            <p style="margin: 5px 0;"><strong>Host:</strong> <code>_rustcare-verify.{}</code></p>
            <p style="margin: 5px 0;"><strong>Type:</strong> TXT</p>
            <p style="margin: 5px 0;"><strong>Value:</strong> <code style="background: #f4f4f4; padding: 2px 6px; border-radius: 3px;">{}</code></p>
        </div>
        
        <div style="background: #fff3cd; padding: 15px; border-left: 4px solid #ffc107; margin: 20px 0;">
            <p style="margin: 0;"><strong>⚠️ DNS Propagation:</strong> It may take up to 48 hours for DNS records to propagate globally.</p>
        </div>
        
        <h3 style="color: #667eea;">Recommended: Additional Email Authentication Records</h3>
        
        <p>For best email deliverability, also add:</p>
        
        <ul style="line-height: 2;">
            <li><strong>SPF Record:</strong> Authorize RustCare mail servers</li>
            <li><strong>DKIM Record:</strong> Email signature verification</li>
            <li><strong>DMARC Record:</strong> Email authentication policy</li>
        </ul>
        
        <p style="margin-top: 30px;">Once DNS records are added, click the verification button in your dashboard.</p>
        
        <p>Best regards,<br>The RustCare Team</p>
        
        <hr style="border: none; border-top: 1px solid #ddd; margin: 30px 0;">
        
        <p style="font-size: 12px; color: #666; text-align: center;">
            This is an automated message from RustCare Engine<br>
            © 2024 RustCare. All rights reserved.
        </p>
    </div>
</body>
</html>
            "#,
            domain, domain, verification_token
        );

        info!(
            organization = org_name,
            domain = domain,
            email = to_email,
            "Sending email domain verification instructions"
        );

        self.send_html_email(to_email, &subject, &body).await
    }

    /// Internal method to send a constructed message using Stalwart mail-send
    async fn send_message(&self, message: MessageBuilder<'_>) -> EmailResult<String> {
        // Build the SMTP client
        let mut smtp_client = SmtpClientBuilder::new(self.config.smtp_host.as_str(), self.config.smtp_port)
            .implicit_tls(false);

        // Add credentials if provided
        if !self.config.smtp_username.is_empty() {
            let username = self.config.smtp_username.as_str();
            let password = self.config.smtp_password.as_str();
            smtp_client = smtp_client.credentials((username, password));
        }

        // Connect to SMTP server
        let mut client = smtp_client
            .connect()
            .await
            .map_err(|e| EmailError::SendFailed(format!("SMTP connection failed: {}", e)))?;

        // Send the message
        let message_id = Uuid::new_v4().to_string();
        client
            .send(message)
            .await
            .map_err(|e| EmailError::SendFailed(format!("Failed to send email: {}", e)))?;

        debug!(message_id = %message_id, "Email sent successfully via Stalwart");
        Ok(message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_from_env() {
        std::env::set_var("SMTP_HOST", "mail.example.com");
        std::env::set_var("SMTP_PORT", "587");
        std::env::set_var("EMAIL_ENABLED", "true");

        let config = EmailConfig::from_env().unwrap();
        assert_eq!(config.smtp_host, "mail.example.com");
        assert_eq!(config.smtp_port, 587);
        assert!(config.email_enabled);
    }
}