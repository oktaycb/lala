//! RAG: korpus yükleme, parçalama, embedding/BM25 arama, indeks kalıcılığı.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::llama::LlamaClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChunk {
    pub source: String,
    pub text: String,
    pub hash: u64,
    #[serde(default)]
    pub vector: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    mode: String,
    chunks: Vec<StoredChunk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Embed,
    Lexical,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub source: String,
    pub text: String,
    pub score: f32,
}

pub struct Index {
    mode: SearchMode,
    chunks: Vec<StoredChunk>,
    tfs: Vec<HashMap<String, u32>>,
    df: HashMap<String, usize>,
    avgdl: f32,
}

pub fn hash_str(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub fn load_corpus(dir: &Path) -> Result<Vec<(String, String)>> {
    if !dir.exists() {
        anyhow::bail!("korpus dizini yok: {}", dir.display());
    }
    let mut out = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(ext.as_str(), "md" | "txt") {
            let content =
                fs::read_to_string(&p).with_context(|| format!("okunamadı: {}", p.display()))?;
            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("belge")
                .to_string();
            out.push((name, content));
        }
    }
    Ok(out)
}

/// Paragraf tabanlı parçalama: hedef ~1000 karakter, tek paragraf aşırı
/// uzunsa kelime sınırında sert bölme.
fn chunk_content(text: &str) -> Vec<String> {
    const TARGET: usize = 1000;
    const HARD: usize = 1400;
    let mut chunks = Vec::new();
    let mut cur = String::new();
    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if para.chars().count() > HARD {
            if !cur.is_empty() {
                chunks.push(std::mem::take(&mut cur));
            }
            let mut piece = String::new();
            for word in para.split_whitespace() {
                if piece.chars().count() + word.chars().count() + 1 > HARD && !piece.is_empty() {
                    chunks.push(std::mem::take(&mut piece));
                }
                if !piece.is_empty() {
                    piece.push(' ');
                }
                piece.push_str(word);
            }
            if !piece.is_empty() {
                chunks.push(piece);
            }
        } else if cur.chars().count() + para.chars().count() + 2 > TARGET && !cur.is_empty() {
            chunks.push(std::mem::take(&mut cur));
            cur.push_str(para);
        } else {
            if !cur.is_empty() {
                cur.push_str("\n\n");
            }
            cur.push_str(para);
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks.retain(|c| !c.trim().is_empty());
    chunks
}

/// Basit unicode-duyarlı tokenleştirici (Türkçe karakterler korunur).
/// Not: 'İ'.to_lowercase() "i" + birleşen nokta üretir; nokta işareti
/// alfanümerik olmadığı için süzülür.
