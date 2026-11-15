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

