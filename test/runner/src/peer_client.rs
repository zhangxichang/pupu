use std::time::Duration;

use eyre::{Result, eyre};
use serde::Serialize;
use serde::de::DeserializeOwned;
use test_protocol::{
    ContactAccept, ContactRequest, Identity, Netem, OkResponse, PeerState, SendMessage,
};

#[derive(Clone)]
pub(crate) struct PeerClient {
    base: String,
    http: reqwest::Client,
}

impl PeerClient {
    pub(crate) fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub(crate) async fn wait_healthy(&self) -> Result<()> {
        for _ in 0..240 {
            if self.health().await.unwrap_or(false) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err(eyre!("节点未在 60 秒内就绪：{}", self.base))
    }

    async fn health(&self) -> Result<bool> {
        let response = self.http.get(self.url("/health")).send().await?;
        Ok(response.status().is_success())
    }

    pub(crate) async fn identity(&self) -> Result<Identity> {
        self.get("/identity").await
    }

    pub(crate) async fn state(&self) -> Result<PeerState> {
        self.get("/state").await
    }

    pub(crate) async fn contact_request(&self, target: &str, message: &str) -> Result<()> {
        let _: OkResponse = self
            .post(
                "/contact/request",
                &ContactRequest {
                    target: target.to_string(),
                    message: message.to_string(),
                },
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn contact_accept(&self, target: &str) -> Result<()> {
        let _: OkResponse = self
            .post(
                "/contact/accept",
                &ContactAccept {
                    target: target.to_string(),
                },
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn send_message(&self, target: &str, markdown: &str) -> Result<()> {
        let _: OkResponse = self
            .post(
                "/message",
                &SendMessage {
                    target: target.to_string(),
                    markdown: markdown.to_string(),
                },
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn netem(&self, netem: Netem) -> Result<()> {
        let _: OkResponse = self.post("/netem", &netem).await?;
        Ok(())
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self.http.get(self.url(path)).send().await?;
        let response = response.error_for_status()?;
        Ok(response.json().await?)
    }

    async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let response = self.http.post(self.url(path)).json(body).send().await?;
        let response = response.error_for_status()?;
        Ok(response.json().await?)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}
