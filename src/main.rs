//! Lala — yerel SLM için Rust agent loop.
//!
//! llama.cpp (llama-server) üstünde çalışan Qwen3 gibi küçük bir modele
//! araç kullanımı katan minimal ajan: web_search (SearXNG), retrieve (RAG)
//! ve fetch_url araçları; grammar-constrained decoding desteği.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lala::agent;
use lala::config::Config;
use lala::llama::{LlamaClient, Message};
use lala::rag;
use lala::tools;
use std::collections::HashMap;
use std::io::Write;

#[derive(Parser)]
#[command(name = "lala", version, about = "Lala — küçük, keskin, yerel LLM ajanı")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Etkileşimli sohbet (REPL)
    Chat {
        /// Korpus dizini (varsayılan: ./corpus veya LALA_CORPUS)
        #[arg(long)]
        corpus: Option<String>,
    },
    /// Tek seferlik soru-cevap
    Ask {
        #[arg(long)]
        corpus: Option<String>,
        /// Soru
        question: Vec<String>,
    },
    /// Korpus indeksini oluştur/güncelle
    Index {
        #[arg(long)]
        corpus: Option<String>,
    },
    /// Grammar-constrained decoding demosu: şemaya zorunlu JSON çıktısı
    Extract {
        /// Çıkarım yapılacak metin
        text: String,
        /// JSON şema dosyası
        #[arg(long)]
        schema: String,
    },
    /// Sunucu ve korpus durumunu denetle
    Doctor,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("Hata: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Doctor => doctor().await,
        Commands::Index { corpus } => {
            let cfg = Config::from_env(corpus);
            index_cmd(&cfg).await
        }
        Commands::Chat { corpus } => {
            let cfg = Config::from_env(corpus);
            chat_cmd(&cfg).await
        }
        Commands::Ask { corpus, question } => {
            let cfg = Config::from_env(corpus);
            let q = question.join(" ");
            if q.trim().is_empty() {
                anyhow::bail!("Soru boş. Kullanım: lala ask \"soru\"");
            }
            ask_cmd(&cfg, q.trim()).await
        }
        Commands::Extract { text, schema } => extract_cmd(&text, &schema).await,
    }
}

fn ensure_client(cfg: &Config) -> LlamaClient {
    LlamaClient::new(cfg.chat_url.clone(), cfg.embed_url.clone())
}

async fn ensure_index(cfg: &Config, client: &LlamaClient) -> Result<rag::Index> {
    if cfg.index_path.exists() {
        match rag::load(&cfg.index_path) {
            Ok(idx) => {
                println!(
                    "[indeks] yüklendi: {} parça, mod={}",
                    idx.len(),
                    idx.mode_name()
                );
                return Ok(idx);
            }
            Err(e) => println!("[indeks] okunamadı ({e}); yeniden oluşturuluyor…"),
        }
    }
    let idx = rag::build(&cfg.corpus_dir, Some(client), &HashMap::new()).await?;
    rag::save(&idx, &cfg.index_path)?;
    println!(
        "[indeks] oluşturuldu: {} parça, mod={} → {}",
        idx.len(),
        idx.mode_name(),
        cfg.index_path.display()
    );
    Ok(idx)
}

