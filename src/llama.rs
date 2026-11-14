//! llama-server (OpenAI uyumlu endpoint) istemcisi.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: Some(text.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(text.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn tool_result(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(text.into()),
            tool_calls: None,
            tool_call_id: Some(id.into()),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

pub struct LlamaClient {
    http: reqwest::Client,
    chat_url: String,
    embed_url: String,
}

impl LlamaClient {
    pub fn new(chat_url: String, embed_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest client");
        Self {
            http,
            chat_url,
            embed_url,
        }
    }

    /// Sohbet tamamlama. `tools` verilirse tool_choice=auto ile gönderilir;
    /// `response_format` verilirse (json_schema) grammar-constrained decoding uygulanır.
    pub async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&Value>,
        response_format: Option<&Value>,
        max_tokens: u32,
    ) -> Result<(Message, Option<Usage>)> {
        let mut body = serde_json::json!({
            "model": "lala",
            "messages": messages,
            "max_tokens": max_tokens,
            // Küçük modellerde araç seçimi ve sadakat için düşük sıcaklık.
            "temperature": 0.4,
        });
        if let Some(t) = tools {
            body["tools"] = t.clone();
            body["tool_choice"] = serde_json::json!("auto");
        }
        if let Some(rf) = response_format {
            body["response_format"] = rf.clone();
        }
        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.chat_url))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("chat sunucusuna ulaşılamadı: {}", self.chat_url))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!(
                "chat sunucusu hata döndürdü ({}): {}",
                status,
                truncate_str(&text, 400)
            );
        }
        let parsed: ChatResponse = serde_json::from_str(&text)
            .with_context(|| format!("chat yanıtı çözümlenemedi: {}", truncate_str(&text, 300)))?;
        let msg = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .context("chat yanıtı boş")?;
        Ok((msg, parsed.usage))
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub async fn health(&self) -> Result<()> {
        let r = self
            .http
            .get(format!("{}/health", self.chat_url))
            .send()
            .await
            .with_context(|| format!("ulaşılamadı: {}", self.chat_url))?;
        if !r.status().is_success() {
            anyhow::bail!("sağlık kontrolü başarısız: {}", r.status());
        }
        Ok(())
    }

    pub async fn embed_alive(&self) -> bool {
        self.http
            .get(format!("{}/health", self.embed_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Toplu embedding. Girdi sırası korunur.
