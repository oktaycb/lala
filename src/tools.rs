//! Araçlar: web_search (SearXNG), retrieve (RAG), fetch_url.
//!
//! Şemalar sabit sırada üretilir — sohbet geçmişinde önek kararlılığı
//! llama-server'ın KV-cache hit oranını korur.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

use crate::html;
use crate::llama::{truncate_str, LlamaClient};
use crate::rag::Index;

/// OpenAI "tools" dizisi — sıra sabit (KV-cache öneki).
pub fn definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "retrieve",
                "description": "Kullanıcının yerel belgelerinde (korpus) arama yapar. Proje veya belge sorularında önce bu aracı kullan.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Arama sorgusu" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Web'de güncel bilgi arar (SearXNG üzerinden). Sorguyu kısa, anahtar-kelime odaklı yaz.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Web arama sorgusu" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "fetch_url",
                "description": "Verilen URL'deki sayfanın metin içeriğini getirir. web_search sonucunda bir sayfayı derinlemesine okumak için kullan.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Getirilecek sayfanın URL'si" }
                    },
                    "required": ["url"]
                }
            }
        }
    ])
}

pub struct ToolCtx<'a> {
    pub http: &'a reqwest::Client,
    pub index: &'a Index,
    pub searxng_url: &'a str,
    pub top_k: usize,
    pub embed: Option<&'a LlamaClient>,
}

pub async fn execute(name: &str, args: &Value, ctx: &ToolCtx<'_>) -> Result<String> {
    match name {
        "retrieve" => tool_retrieve(args, ctx).await,
        "web_search" => tool_web_search(args, ctx).await,
        "fetch_url" => tool_fetch_url(args, ctx).await,
        other => anyhow::bail!("bilinmeyen araç: {other}"),
    }
}

async fn tool_retrieve(args: &Value, ctx: &ToolCtx<'_>) -> Result<String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .context("'query' eksik")?;
    let hits = ctx.index.search(query, ctx.top_k, ctx.embed).await?;
    if hits.is_empty() {
        return Ok(json!({"sonuclar": [], "not": "korpusta eşleşme bulunamadı"}).to_string());
    }
    let results: Vec<Value> = hits
        .iter()
        .map(|h| {
            json!({
                "kaynak": h.source,
                "skor": (h.score * 1000.0).round() / 1000.0,
                "metin": truncate_str(&h.text, 500),
            })
        })
        .collect();
    Ok(json!({ "sonuclar": results }).to_string())
}

