use crate::imap::ConnectionSecurity;
use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use pebble_core::{PebbleError, Result};

pub struct SmtpSender {
    host: String,
    port: u16,
    credentials: Credentials,
    security: ConnectionSecurity,
}

impl SmtpSender {
    pub fn new(host: String, port: u16, username: String, password: String, security: ConnectionSecurity) -> Self {
        Self {
            host,
            port,
            credentials: Credentials::new(username, password),
            security,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send(
        &self,
        from: &str,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        in_reply_to: Option<&str>,
    ) -> Result<()> {
        if to.is_empty() {
            return Err(PebbleError::Internal("No recipients".to_string()));
        }

        let from_mailbox: Mailbox = from
            .parse()
            .map_err(|e| PebbleError::Internal(format!("Invalid from address: {e}")))?;

        let mut builder = lettre::Message::builder()
            .from(from_mailbox)
            .subject(subject);

        for addr in to {
            let mailbox: Mailbox = addr
                .parse()
                .map_err(|e| PebbleError::Internal(format!("Invalid to address '{addr}': {e}")))?;
            builder = builder.to(mailbox);
        }

        for addr in cc {
            let mailbox: Mailbox = addr
                .parse()
                .map_err(|e| PebbleError::Internal(format!("Invalid cc address '{addr}': {e}")))?;
            builder = builder.cc(mailbox);
        }

        for addr in bcc {
            let mailbox: Mailbox = addr
                .parse()
                .map_err(|e| PebbleError::Internal(format!("Invalid bcc address '{addr}': {e}")))?;
            builder = builder.bcc(mailbox);
        }

        if let Some(reply_to) = in_reply_to {
            builder = builder.in_reply_to(reply_to.to_string());
        }

        let email = if let Some(html) = body_html {
            builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .content_type(ContentType::TEXT_PLAIN)
                                .body(body_text.to_string()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .content_type(ContentType::TEXT_HTML)
                                .body(html.to_string()),
                        ),
                )
                .map_err(|e| PebbleError::Internal(format!("Failed to build email: {e}")))?
        } else {
            builder
                .body(body_text.to_string())
                .map_err(|e| PebbleError::Internal(format!("Failed to build email: {e}")))?
        };

        let transport = match self.security {
            ConnectionSecurity::Tls => {
                SmtpTransport::relay(&self.host)
                    .map_err(|e| PebbleError::Network(format!("SMTP relay error: {e}")))?
                    .port(self.port)
                    .credentials(self.credentials.clone())
                    .build()
            }
            ConnectionSecurity::StartTls => {
                SmtpTransport::starttls_relay(&self.host)
                    .map_err(|e| PebbleError::Network(format!("SMTP STARTTLS error: {e}")))?
                    .port(self.port)
                    .credentials(self.credentials.clone())
                    .build()
            }
            ConnectionSecurity::Plain => {
                SmtpTransport::builder_dangerous(&self.host)
                    .port(self.port)
                    .credentials(self.credentials.clone())
                    .build()
            }
        };

        transport
            .send(&email)
            .map_err(|e| PebbleError::Network(format!("SMTP send failed: {e}")))?;

        Ok(())
    }
}
