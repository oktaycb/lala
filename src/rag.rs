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
