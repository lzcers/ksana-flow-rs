use anyhow::Result;
use async_trait::async_trait;
use flow::{Context, Node};
use lettre::{
    Message, SmtpTransport, Transport,
    transport::smtp::authentication::{Credentials, Mechanism},
};
use tracing::{error, info};

use crate::config::get_config;

pub fn email_notify(subject: &str, body: &str) -> Result<()> {
    // 收件人邮箱
    let Ok(config) = get_config() else {
        error!("The Email config is not failed");
        return Ok(());
    };
    let email_receiver = "zack@ksana.net";

    let email = Message::builder()
        .from("策略触发提醒 <zack@ksana.net>".parse()?)
        .to(email_receiver.parse()?)
        .subject(subject)
        .body(String::from(body))?;

    let sender = SmtpTransport::starttls_relay(&config.email.smtp_server)?
        // Add credentials for authentication
        .credentials(Credentials::new(
            config.email.email.to_owned(),
            config.email.email_token.to_owned(),
        ))
        // Configure expected authentication mechanism
        .authentication(vec![Mechanism::Plain])
        // Connection pool settings
        .build();

    let result = sender.send(&email);

    match result {
        Ok(_) => info!("The Email report was sent Successfully."),
        Err(e) => error!("The Email report was sent failed: {:?}", e),
    }

    Ok(())
}

#[derive(Default)]
pub struct EmailNotifyNode {
    subject: String,
    body: String,
}

impl EmailNotifyNode {
    pub fn new(subject: String, body: String) -> Self {
        Self { subject, body }
    }
}

#[async_trait]
impl Node for EmailNotifyNode {
    type In = ();
    type Out = ();

    async fn run(&mut self, _ctx: &Context, _input: Self::In) -> Self::Out {
        if let Err(e) = email_notify(&self.subject, &self.body) {
            error!("EmailNotifyNode failed to send email: {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_email_notify_node() -> Result<()> {
        let mut node = EmailNotifyNode::new("Test Subject".to_string(), "Test Body".to_string());
        let ctx = Context::new();
        node.run(&ctx, ()).await;
        Ok(())
    }

    #[test]
    fn test_email_send() -> Result<()> {
        email_notify("Buy", "buy")?;
        Ok(())
    }
}
