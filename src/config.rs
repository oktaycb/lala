use std::path::PathBuf;

/// Çalışma zamanı yapılandırması: ortam değişkenlerinden, makul varsayılanlarla.
///
/// LALA_CHAT_URL      — llama-server (sohbet modeli)   [http://127.0.0.1:8123]
/// LALA_EMBED_URL     — llama-server (embedding modeli) [http://127.0.0.1:8125]
/// LALA_SEARXNG_URL   — SearXNG örneği                  [http://127.0.0.1:8888]
/// LALA_CORPUS        — RAG korpus dizini               [./corpus]
#[derive(Debug, Clone)]
pub struct Config {
    pub chat_url: String,
    pub embed_url: String,
    pub searxng_url: String,
    pub corpus_dir: PathBuf,
    pub index_path: PathBuf,
    pub top_k: usize,
    pub max_iterations: usize,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).ok().filter(|s| !s.is_empty()).unwrap_or_else(|| default.to_string())
}

