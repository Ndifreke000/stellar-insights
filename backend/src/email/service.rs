use lettre::{Message, SmtpTransport, Transport};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;

pub struct EmailService {
    smtp_host: String,
    smtp_user: String,
    smtp_pass: String,
}

impl EmailService {
    pub fn new(smtp_host: String, smtp_user: String, smtp_pass: String) -> Self {
        Self { smtp_host, smtp_user, smtp_pass }
    }

    pub fn send_html(&self, to: &str, subject: &str, html: &str) -> anyhow::Result<()> {
        let email = Message::builder()
            .from(self.smtp_user.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html.to_string())?;

        let creds = Credentials::new(self.smtp_user.clone(), self.smtp_pass.clone());
        let mailer = SmtpTransport::relay(&self.smtp_host)?
            .credentials(creds)
            .build();

        mailer.send(&email)?;
        Ok(())
    }
}
