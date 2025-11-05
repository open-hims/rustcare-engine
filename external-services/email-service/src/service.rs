// Email service implementation with multiple provider support
use crate::error::{EmailError, EmailResult};
use mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use uuid::Uuid;

/// Email provider types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailProviderType {
    Smtp,
    Gmail,
    Ses,          // Amazon SES
    SendGrid,
    Mailgun,
    Mailchimp,
    Postmark,
    Resend,
}

/// Email provider configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EmailProvider {
    /// Generic SMTP server
    Smtp {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        use_tls: bool,
    },
    /// Gmail with OAuth2
    Gmail {
        client_id: String,
        client_secret: String,
        refresh_token: String,
        access_token: Option<String>,
    },
    /// Amazon SES
    Ses {
        region: String,
        access_key_id: String,
        secret_access_key: String,
    },
    /// SendGrid
    SendGrid {
        api_key: String,
    },
    /// Mailgun
    Mailgun {
        domain: String,
        api_key: String,
    },
    /// Mailchimp Transactional
    Mailchimp {
        api_key: String,
    },
    /// Postmark
    Postmark {
        api_token: String,
    },
    /// Resend
    Resend {
        api_key: String,
    },
}

/// Email service configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailConfig {
    pub provider: EmailProvider,
    pub from_email: String,
    pub from_name: String,
    pub email_enabled: bool,
}

impl EmailConfig {
    /// Load email configuration from environment variables
    pub fn from_env() -> EmailResult<Self> {
        let email_enabled = std::env::var("EMAIL_ENABLED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        let from_email = std::env::var("EMAIL_FROM")
            .unwrap_or_else(|_| "noreply@rustcare.local".to_string());
        
        let from_name = std::env::var("EMAIL_FROM_NAME")
            .unwrap_or_else(|_| "RustCare Engine".to_string());
        
        // Detect provider from environment
        let provider = if let Ok(provider_type) = std::env::var("EMAIL_PROVIDER") {
            match provider_type.to_lowercase().as_str() {
                "gmail" => {
                    EmailProvider::Gmail {
                        client_id: std::env::var("GMAIL_CLIENT_ID")
                            .unwrap_or_default(),
                        client_secret: std::env::var("GMAIL_CLIENT_SECRET")
                            .unwrap_or_default(),
                        refresh_token: std::env::var("GMAIL_REFRESH_TOKEN")
                            .unwrap_or_default(),
                        access_token: std::env::var("GMAIL_ACCESS_TOKEN").ok(),
                    }
                }
                "ses" => {
                    EmailProvider::Ses {
                        region: std::env::var("AWS_REGION")
                            .unwrap_or_else(|_| "us-east-1".to_string()),
                        access_key_id: std::env::var("AWS_ACCESS_KEY_ID")
                            .unwrap_or_default(),
                        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
                            .unwrap_or_default(),
                    }
                }
                "sendgrid" => {
                    EmailProvider::SendGrid {
                        api_key: std::env::var("SENDGRID_API_KEY")
                            .unwrap_or_default(),
                    }
                }
                "mailgun" => {
                    EmailProvider::Mailgun {
                        domain: std::env::var("MAILGUN_DOMAIN")
                            .unwrap_or_default(),
                        api_key: std::env::var("MAILGUN_API_KEY")
                            .unwrap_or_default(),
                    }
                }
                "mailchimp" => {
                    EmailProvider::Mailchimp {
                        api_key: std::env::var("MAILCHIMP_API_KEY")
                            .unwrap_or_default(),
                    }
                }
                "postmark" => {
                    EmailProvider::Postmark {
                        api_token: std::env::var("POSTMARK_API_TOKEN")
                            .unwrap_or_default(),
                    }
                }
                "resend" => {
                    EmailProvider::Resend {
                        api_key: std::env::var("RESEND_API_KEY")
                            .unwrap_or_default(),
                    }
                }
                _ => EmailProvider::Smtp {
                    host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
                    port: std::env::var("SMTP_PORT")
                        .ok()
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(587),
                    username: std::env::var("SMTP_USERNAME").ok(),
                    password: std::env::var("SMTP_PASSWORD").ok(),
                    use_tls: std::env::var("SMTP_TLS_ENABLED")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                }
            }
        } else {
            // Default to SMTP
            EmailProvider::Smtp {
                host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("SMTP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(587),
                username: std::env::var("SMTP_USERNAME").ok(),
                password: std::env::var("SMTP_PASSWORD").ok(),
                use_tls: std::env::var("SMTP_TLS_ENABLED")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
            }
        };
        
        Ok(Self {
            provider,
            from_email,
            from_name,
            email_enabled,
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
                self.config.from_name.as_str(),
                self.config.from_email.as_str(),
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
                self.config.from_name.as_str(),
                self.config.from_email.as_str(),
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

    /// Send user account credentials (hospital onboarding)
    pub async fn send_user_credentials(
        &self,
        to_email: &str,
        user_name: &str,
        org_name: &str,
        username: &str,
        temporary_password: &str,
        login_url: &str,
    ) -> EmailResult<String> {
        let subject = format!("Welcome to {} - Your RustCare Credentials", org_name);
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
        <h2 style="color: #667eea; margin-top: 0;">Your Account is Ready</h2>
        
        <p>Hello {},</p>
        
        <p>Your account has been successfully created for <strong>{}</strong> on RustCare. Here are your login credentials:</p>
        
        <div style="background: white; padding: 20px; border-left: 4px solid #667eea; margin: 20px 0;">
            <h3 style="margin-top: 0; color: #667eea;">Your Login Credentials</h3>
            <p style="margin: 10px 0;"><strong>Username:</strong> <code style="background: #f4f4f4; padding: 4px 8px; border-radius: 3px; font-family: monospace;">{}</code></p>
            <p style="margin: 10px 0;"><strong>Temporary Password:</strong> <code style="background: #f4f4f4; padding: 4px 8px; border-radius: 3px; font-family: monospace; color: #dc3545;">{}</code></p>
        </div>
        
        <div style="background: #dc3545; color: white; padding: 15px; border-radius: 5px; margin: 20px 0;">
            <p style="margin: 0; font-weight: bold;">🔒 Security Notice:</p>
            <p style="margin: 5px 0 0 0;">This is a temporary password. You <strong>must</strong> change it on your first login!</p>
        </div>
        
        <div style="text-align: center; margin: 30px 0;">
            <a href="{}" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 15px 40px; text-decoration: none; border-radius: 5px; font-weight: bold; display: inline-block;">Login to RustCare</a>
        </div>
        
        <h3 style="color: #667eea;">Getting Started:</h3>
        <ol style="line-height: 2;">
            <li>Click the login button above or visit the RustCare portal</li>
            <li>Enter your username and temporary password</li>
            <li>You will be prompted to change your password immediately</li>
            <li>Explore your dashboard and start using RustCare</li>
        </ol>
        
        <h3 style="color: #667eea;">Need Help?</h3>
        <ul style="line-height: 2;">
            <li>Password reset: Use "Forgot Password" on the login page</li>
            <li>Technical support: Contact your organization administrator</li>
            <li>Documentation: Access help center from your dashboard</li>
        </ul>
        
        <p style="margin-top: 30px;">Best regards,<br><strong>RustCare Team</strong></p>
        
        <hr style="border: none; border-top: 1px solid #ddd; margin: 30px 0;">
        
        <p style="font-size: 12px; color: #666; text-align: center;">
            This is an automated message from RustCare Engine<br>
            Please keep your credentials secure and do not share them with anyone<br>
            © 2024 RustCare. All rights reserved.
        </p>
    </div>
</body>
</html>
            "#,
            user_name, org_name, username, temporary_password, login_url
        );

        info!(
            user_name = user_name,
            organization = org_name,
            email = to_email,
            "Sending user credentials email"
        );

        self.send_html_email(to_email, &subject, &body).await
    }

    /// Test email configuration by checking connection without sending
    pub async fn verify_email_config(&self) -> EmailResult<()> {
        info!("Verifying email configuration");

        match &self.config.provider {
            EmailProvider::Smtp { host, port, username, password, use_tls } => {
                info!(host = %host, port = %port, "Testing SMTP connection");

                // Build SMTP client
                let mut smtp_client = SmtpClientBuilder::new(host.as_str(), *port)
                    .implicit_tls(*use_tls);

                // Add credentials if provided
                if let (Some(user), Some(pass)) = (username, password) {
                    smtp_client = smtp_client.credentials((user.as_str(), pass.as_str()));
                }

                // Attempt to connect (without sending)
                let _client = smtp_client
                    .connect()
                    .await
                    .map_err(|e| EmailError::SendFailed(format!("SMTP connection failed: {}", e)))?;

                info!(provider = "smtp", "Email configuration verified successfully");
                Ok(())
            }
            EmailProvider::Gmail { .. } => {
                info!("Testing Gmail provider connection");
                // Test Gmail SMTP
                let smtp_client = SmtpClientBuilder::new("smtp.gmail.com", 587)
                    .implicit_tls(false);
                
                let _client = smtp_client
                    .connect()
                    .await
                    .map_err(|e| EmailError::SendFailed(format!("Gmail connection failed: {}", e)))?;

                info!(provider = "gmail", "Email configuration verified successfully");
                Ok(())
            }
            EmailProvider::Ses { .. } => {
                // TODO: Verify AWS SES credentials
                info!(provider = "ses", "SES verification not yet implemented");
                Ok(())
            }
            EmailProvider::SendGrid { .. } => {
                // TODO: Verify SendGrid API key
                info!(provider = "sendgrid", "SendGrid verification not yet implemented");
                Ok(())
            }
            EmailProvider::Mailgun { .. } => {
                // TODO: Verify Mailgun API key
                info!(provider = "mailgun", "Mailgun verification not yet implemented");
                Ok(())
            }
            EmailProvider::Mailchimp { .. } => {
                // TODO: Verify Mailchimp API key
                info!(provider = "mailchimp", "Mailchimp verification not yet implemented");
                Ok(())
            }
            EmailProvider::Postmark { .. } => {
                // TODO: Verify Postmark API token
                info!(provider = "postmark", "Postmark verification not yet implemented");
                Ok(())
            }
            EmailProvider::Resend { .. } => {
                // TODO: Verify Resend API key
                info!(provider = "resend", "Resend verification not yet implemented");
                Ok(())
            }
        }
    }

    /// Internal method to send a constructed message using configured provider
    async fn send_message(&self, message: MessageBuilder<'_>) -> EmailResult<String> {
        match &self.config.provider {
            EmailProvider::Smtp { host, port, username, password, use_tls } => {
                // Build SMTP client
                let mut smtp_client = SmtpClientBuilder::new(host.as_str(), *port)
                    .implicit_tls(*use_tls);

                // Add credentials if provided
                if let (Some(user), Some(pass)) = (username, password) {
                    smtp_client = smtp_client.credentials((user.as_str(), pass.as_str()));
                }

                // Connect and send
                let mut client = smtp_client
                    .connect()
                    .await
                    .map_err(|e| EmailError::SendFailed(format!("SMTP connection failed: {}", e)))?;

                let message_id = Uuid::new_v4().to_string();
                client
                    .send(message)
                    .await
                    .map_err(|e| EmailError::SendFailed(format!("Failed to send email: {}", e)))?;

                debug!(provider = "smtp", message_id = %message_id, "Email sent successfully");
                Ok(message_id)
            }
            EmailProvider::Gmail { .. } => {
                // TODO: Implement Gmail OAuth2 sending
                // For now, use SMTP with Gmail credentials
                info!("Gmail provider selected, using SMTP fallback");
                let smtp_client = SmtpClientBuilder::new("smtp.gmail.com", 587)
                    .implicit_tls(false);
                
                // Note: Gmail requires app-specific password when not using OAuth2
                let mut client = smtp_client
                    .connect()
                    .await
                    .map_err(|e| EmailError::SendFailed(format!("SMTP connection failed: {}", e)))?;

                let message_id = Uuid::new_v4().to_string();
                client.send(message).await
                    .map_err(|e| EmailError::SendFailed(format!("Failed to send email: {}", e)))?;

                debug!(provider = "gmail", message_id = %message_id, "Email sent successfully");
                Ok(message_id)
            }
            EmailProvider::Ses { .. } => {
                // TODO: Implement AWS SES sending via SDK
                Err(EmailError::SendFailed("AWS SES provider not yet implemented".to_string()))
            }
            EmailProvider::SendGrid { .. } => {
                // TODO: Implement SendGrid API sending
                Err(EmailError::SendFailed("SendGrid provider not yet implemented".to_string()))
            }
            EmailProvider::Mailgun { .. } => {
                // TODO: Implement Mailgun API sending
                Err(EmailError::SendFailed("Mailgun provider not yet implemented".to_string()))
            }
            EmailProvider::Mailchimp { .. } => {
                // TODO: Implement Mailchimp API sending
                Err(EmailError::SendFailed("Mailchimp provider not yet implemented".to_string()))
            }
            EmailProvider::Postmark { .. } => {
                // TODO: Implement Postmark API sending
                Err(EmailError::SendFailed("Postmark provider not yet implemented".to_string()))
            }
            EmailProvider::Resend { .. } => {
                // TODO: Implement Resend API sending
                Err(EmailError::SendFailed("Resend provider not yet implemented".to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_from_env() {
        std::env::set_var("EMAIL_PROVIDER", "smtp");
        std::env::set_var("SMTP_HOST", "mail.example.com");
        std::env::set_var("SMTP_PORT", "587");
        std::env::set_var("EMAIL_ENABLED", "true");

        let config = EmailConfig::from_env().unwrap();
        assert!(config.email_enabled);
        match config.provider {
            EmailProvider::Smtp { host, port, .. } => {
                assert_eq!(host, "mail.example.com");
                assert_eq!(port, 587);
            }
            _ => panic!("Expected SMTP provider"),
        }
    }
}