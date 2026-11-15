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
fn tokenize(s: &str) -> Vec<String> {
    let mut word = String::new();
    let mut words = Vec::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                if lc.is_alphanumeric() {
                    word.push(lc);
                }
            }
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// Türkçe eklemeli morfoloji için yumuşak eşleşme: tam eşleşme yoksa,
/// kısa terimler dışında ön-ek ilişkisi (başkent ↔ başkentidir) kabul edilir.
fn term_freq(tf: &HashMap<String, u32>, w: &str) -> u32 {
    if let Some(f) = tf.get(w) {
        return *f;
    }
    if w.chars().count() < 4 {
        return 0;
    }
    let mut best = 0u32;
    for (k, f) in tf {
        let kc = k.chars().count();
        if kc < 4 {
            continue;
        }
        if k.starts_with(w) || w.starts_with(k) {
            best = best.max(*f);
        }
    }
    best
}

fn doc_freq(df: &HashMap<String, usize>, w: &str) -> usize {
    if let Some(d) = df.get(w) {
        return *d;
    }
    if w.chars().count() < 4 {
        return 0;
    }
    let mut total = 0usize;
    for (k, d) in df {
        let kc = k.chars().count();
        if kc < 4 {
            continue;
        }
        if k.starts_with(w) || w.starts_with(k) {
            total += d;
        }
    }
    total
}

/// Korpusu okuyup indeks kurar. Embedding sunucusu varsa vektörlü mod,
/// yoksa (ve tüm vektörler cache'ten gelmiyorsa) BM25 moduna düşer.
pub async fn build(
    dir: &Path,
    client: Option<&LlamaClient>,
    cache: &HashMap<u64, Vec<f32>>,
) -> Result<Index> {
    let docs = load_corpus(dir)?;
    anyhow::ensure!(
        !docs.is_empty(),
        "korpus boş: {} içinde .md/.txt yok",
        dir.display()
    );
    let mut chunks = Vec::new();
    for (name, content) in &docs {
        for (i, c) in chunk_content(content).into_iter().enumerate() {
            let hash = hash_str(&c);
            let vector = cache.get(&hash).cloned().unwrap_or_default();
            chunks.push(StoredChunk {
                source: format!("{name}#{}", i + 1),
                text: c,
                hash,
                vector,
            });
        }
    }
    let mut mode = SearchMode::Lexical;
    if let Some(client) = client {
        let missing = chunks.iter().filter(|c| c.vector.is_empty()).count();
        if client.embed_alive().await {
            embed_missing(&mut chunks, client).await?;
            mode = SearchMode::Embed;
        } else if missing < chunks.len() {
            mode = SearchMode::Embed;
        }
    }
    Ok(Index::new(mode, chunks))
}

async fn embed_missing(chunks: &mut [StoredChunk], client: &LlamaClient) -> Result<()> {
    let mut done = 0usize;
    while done < chunks.len() {
        let end = (done + 16).min(chunks.len());
        let need: Vec<usize> = (done..end).filter(|&i| chunks[i].vector.is_empty()).collect();
        if need.is_empty() {
            done = end;
            continue;
        }
        let inputs: Vec<String> = need.iter().map(|&i| chunks[i].text.clone()).collect();
        let vecs = client.embed_batch(&inputs).await?;
        for (i, v) in need.into_iter().zip(vecs) {
            chunks[i].vector = v;
        }
        done = end;
    }
    Ok(())
}

impl Index {
    pub fn new(mode: SearchMode, chunks: Vec<StoredChunk>) -> Self {
        let mut tfs: Vec<HashMap<String, u32>> = vec![HashMap::new(); chunks.len()];
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut total_len = 0usize;
        if mode == SearchMode::Lexical {
            for (i, c) in chunks.iter().enumerate() {
                let mut tf: HashMap<String, u32> = HashMap::new();
                for w in tokenize(&c.text) {
                    *tf.entry(w).or_insert(0) += 1;
                }
                for w in tf.keys() {
                    *df.entry(w.clone()).or_insert(0) += 1;
                }
                total_len += tf.values().sum::<u32>() as usize;
                tfs[i] = tf;
            }
        }
        let avgdl = if chunks.is_empty() {
            1.0
        } else {
            (total_len as f32 / chunks.len() as f32).max(1.0)
        };
        Self {
            mode,
            chunks,
            tfs,
            df,
            avgdl,
        }
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            SearchMode::Embed => "embed",
            SearchMode::Lexical => "lexical",
        }
    }

    pub async fn search(
        &self,
        query: &str,
        k: usize,
        client: Option<&LlamaClient>,
    ) -> Result<Vec<Hit>> {
        if self.chunks.is_empty() {
            return Ok(vec![]);
        }
        match self.mode {
            SearchMode::Embed => {
                let client =
                    client.context("embedding modu için embedding sunucusu gerekli")?;
                let qtext = format!(
                    "Instruct: Verilen sorgu için ilgili belge bölümlerini bul\nQuery: {query}"
                );
                let qv = client
                    .embed_batch(&[qtext])
                    .await?
                    .pop()
                    .context("sorgu embedding'i boş")?;
                Ok(self.rank_cosine(&qv, k))
            }
            SearchMode::Lexical => Ok(self.rank_bm25(query, k)),
        }
    }

    fn rank_cosine(&self, qv: &[f32], k: usize) -> Vec<Hit> {
        let mut scored: Vec<(usize, f32)> = Vec::new();
        for (i, c) in self.chunks.iter().enumerate() {
            if c.vector.len() != qv.len() {
                continue;
            }
            let mut dot = 0.0f32;
            let mut na = 0.0f32;
            let mut nb = 0.0f32;
            for j in 0..qv.len() {
                dot += qv[j] * c.vector[j];
                na += qv[j] * qv[j];
                nb += c.vector[j] * c.vector[j];
            }
            if na > 0.0 && nb > 0.0 {
                scored.push((i, dot / (na.sqrt() * nb.sqrt())));
            }
        }
        Self::top_hits(&self.chunks, &mut scored, k)
    }

    fn rank_bm25(&self, query: &str, k: usize) -> Vec<Hit> {
        let q = tokenize(query);
        if q.is_empty() {
            return vec![];
        }
        let n = self.chunks.len() as f32;
        let (k1, b) = (1.2f32, 0.75f32);
        let mut scored: Vec<(usize, f32)> = Vec::new();
        for (i, tf) in self.tfs.iter().enumerate() {
            let mut score = 0.0f32;
            let dl = tf.values().sum::<u32>() as f32;
            for w in &q {
                let f = term_freq(tf, w) as f32;
                if f == 0.0 {
                    continue;
                }
                let dfn = doc_freq(&self.df, w) as f32;
                let idf = ((n - dfn + 0.5) / (dfn + 0.5) + 1.0).ln();
                score += idf * (f * (k1 + 1.0))
                    / (f + k1 * (1.0 - b + b * (dl / self.avgdl)));
            }
            if score > 0.0 {
                scored.push((i, score));
            }
        }
        Self::top_hits(&self.chunks, &mut scored, k)
    }

    fn top_hits(
        chunks: &[StoredChunk],
        scored: &mut Vec<(usize, f32)>,
        k: usize,
    ) -> Vec<Hit> {
        scored
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .iter()
            .take(k)
            .map(|(i, s)| Hit {
                source: chunks[*i].source.clone(),
                text: chunks[*i].text.clone(),
                score: *s,
            })
            .collect()
    }
}

