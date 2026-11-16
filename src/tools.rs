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

async fn tool_web_search(args: &Value, ctx: &ToolCtx<'_>) -> Result<String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .context("'query' eksik")?;
    let url = format!("{}/search", ctx.searxng_url.trim_end_matches('/'));
    let resp = ctx
        .http
        .get(&url)
        .query(&[
            ("q", query),
            ("format", "json"),
            ("language", "tr"),
            ("safesearch", "1"),
        ])
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .context("SearXNG'ye ulaşılamadı (docker ile başlat: docker run -p 8888:8080 searxng/searxng)")?;
    if !resp.status().is_success() {
        anyhow::bail!("SearXNG hata döndürdü: {} (format=json kapalı olabilir)", resp.status());
    }
    let v: Value = resp.json().await.context("SearXNG yanıtı JSON değil")?;
    let mut out = Vec::new();
    if let Some(results) = v.get("results").and_then(|r| r.as_array()) {
        for r in results.iter().take(5) {
            out.push(json!({
                "baslik": r.get("title").and_then(|t| t.as_str()).unwrap_or(""),
                "url": r.get("url").and_then(|t| t.as_str()).unwrap_or(""),
                "ozet": truncate_str(r.get("content").and_then(|t| t.as_str()).unwrap_or(""), 220),
            }));
            if out.len() >= 5 {
                break;
            }
        }
    }
    Ok(json!({ "sonuclar": out }).to_string())
}

pub async fn probe_searxng(base: &str) -> Result<()> {
    let http = reqwest::Client::new();
    let url = format!("{}/search", base.trim_end_matches('/'));
    let resp = http
        .get(&url)
        .query(&[("q", "test"), ("format", "json")])
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("durum: {}", resp.status());
    }
    Ok(())
}

async fn tool_fetch_url(args: &Value, ctx: &ToolCtx<'_>) -> Result<String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .context("'url' eksik")?;
    let resp = ctx
        .http
        .get(url)
        .header("User-Agent", "lala/0.1 (yerel ajan)")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("sayfa indirilemedi")?;
    if !resp.status().is_success() {
        anyhow::bail!("sayfa hata döndürdü: {}", resp.status());
    }
    let body = resp.text().await.context("sayfa gövdesi okunamadı")?;
    let text = if body.trim_start().starts_with('<') {
        html::strip_html(&body)
    } else {
        body
    };
    Ok(json!({
        "url": url,
        "metin": truncate_str(&text, 2400),
    })
    .to_string())
}

