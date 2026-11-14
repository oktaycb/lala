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

impl Config {
    pub fn from_env(corpus_override: Option<String>) -> Self {
        let corpus_dir = corpus_override
            .map(PathBuf::from)
            .or_else(|| std::env::var("LALA_CORPUS").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("corpus"));
        let index_path = corpus_dir.join(".lala-index.json");
        Self {
            chat_url: env_or("LALA_CHAT_URL", "http://127.0.0.1:8123"),
            embed_url: env_or("LALA_EMBED_URL", "http://127.0.0.1:8125"),
            searxng_url: env_or("LALA_SEARXNG_URL", "http://127.0.0.1:8888"),
            corpus_dir,
            index_path,
            top_k: 4,
            max_iterations: 6,
        }
    }
}
