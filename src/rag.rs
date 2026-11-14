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

