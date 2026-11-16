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

async fn chat_cmd(cfg: &Config) -> Result<()> {
    let client = ensure_client(cfg);
    let index = ensure_index(cfg, &client).await?;
    let mut messages = vec![Message::system(agent::SYSTEM_PROMPT)];
    println!("Lala hazır. (yardım için /yardim, çıkış için /cik)");
    let stdin = std::io::stdin();
    loop {
        print!("Sen> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        match line {
            "" => continue,
            "/cik" | "/quit" => break,
            "/sifirla" => {
                messages = vec![Message::system(agent::SYSTEM_PROMPT)];
                println!("[sohbet sıfırlandı]");
                continue;
            }
            "/araclar" => {
                println!("Araçlar: web_search, retrieve, fetch_url");
                continue;
            }
            "/yardim" | "/help" => {
                println!("/araclar araçları listeler, /sifirla sohbeti sıfırlar, /cik çıkar.");
                continue;
            }
            _ => {}
        }
        messages.push(Message::user(line));
        match agent::run_turn(cfg, &client, &index, &mut messages, 1500).await {
            Ok((answer, sources)) => {
                println!("\nLala> {answer}\n");
                if !sources.is_empty() {
                    println!("Kaynaklar:");
                    for (i, s) in sources.iter().enumerate() {
                        println!("  [{}] {}", i + 1, s);
                    }
                    println!();
                }
            }
            Err(e) => {
                // Son kullanıcı mesajını tut, hatayı bildir.
                eprintln!("[hata] {e:#}");
            }
        }
    }
    Ok(())
}

async fn ask_cmd(cfg: &Config, question: &str) -> Result<()> {
    let client = ensure_client(cfg);
    let index = ensure_index(cfg, &client).await?;
    let mut messages = vec![
        Message::system(agent::SYSTEM_PROMPT),
        Message::user(question),
    ];
    let (answer, sources) =
        agent::run_turn(cfg, &client, &index, &mut messages, 1500).await?;
    println!("{answer}\n");
    if !sources.is_empty() {
        println!("Kaynaklar:");
        for (i, s) in sources.iter().enumerate() {
            println!("  [{}] {}", i + 1, s);
        }
    }
    Ok(())
}

async fn index_cmd(cfg: &Config) -> Result<()> {
    let client = ensure_client(cfg);
    // Vektör cache'i: aynı metinler yeniden gömülmesin.
    let cache = if cfg.index_path.exists() {
        rag::load(&cfg.index_path)
            .map(|idx| rag::cache_map(&idx))
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    let idx = rag::build(&cfg.corpus_dir, Some(&client), &cache).await?;
    let n = idx.len();
    let mode = idx.mode_name().to_string();
    rag::save(&idx, &cfg.index_path)?;
    println!(
        "İndeks oluşturuldu: {n} parça, mod={mode} → {}",
        cfg.index_path.display()
    );
    Ok(())
}

async fn extract_cmd(text: &str, schema_path: &str) -> Result<()> {
    let client = ensure_client(&Config::from_env(None));
    let raw = std::fs::read_to_string(schema_path)
        .with_context(|| format!("şema okunamadı: {schema_path}"))?;
    let schema: serde_json::Value =
        serde_json::from_str(raw.trim()).context("şema geçerli JSON değil")?;
    let rf = serde_json::json!({
        "type": "json_schema",
        "json_schema": { "name": "cevap", "strict": true, "schema": schema }
    });
    let messages = vec![
        Message::system("/no_think Şemaya uyan tek bir JSON nesnesi döndür."),
        Message::user(text),
    ];
    let (msg, usage) = client.chat(&messages, None, Some(&rf), 800).await?;
    let content = msg.content.unwrap_or_default();
    println!("{content}");
    match serde_json::from_str::<serde_json::Value>(content.trim()) {
        Ok(_) => println!("\n[✓ çıktı geçerli JSON; usage: {usage:?}]"),
        Err(e) => println!("\n[!] JSON parse uyarısı: {e}"),
    }
    Ok(())
}

async fn doctor() -> Result<()> {
    let cfg = Config::from_env(None);
    let client = ensure_client(&cfg);
    println!("chat  : {} → ", cfg.chat_url);
    match client.health().await {
        Ok(_) => println!("        ✓ çalışıyor"),
        Err(e) => println!("        ✗ erişilemedi: {e}"),
    }
    print!("embed : {} → ", cfg.embed_url);
    if client.embed_alive().await {
        println!("✓ çalışıyor (RAG embedding modu mümkün)");
    } else {
        println!("✗ kapalı (RAG lexical/BM25 moduna düşer)");
    }
    print!("searxng: {} → ", cfg.searxng_url);
    match tools::probe_searxng(&cfg.searxng_url).await {
        Ok(_) => println!("✓ çalışıyor"),
        Err(_) => println!("✗ kapalı (web_search aracı hata mesajı döndürür)"),
    }
    println!(
        "korpus: {} → ",
        cfg.corpus_dir.display()
    );
    match rag::load_corpus(&cfg.corpus_dir) {
        Ok(docs) => println!("        ✓ {} belge", docs.len()),
        Err(e) => println!("        ✗ okunamadı: {e}"),
    }
    print!("indeks: {} → ", cfg.index_path.display());
    if cfg.index_path.exists() {
        match rag::load(&cfg.index_path) {
            Ok(idx) => println!("✓ {} parça, mod={}", idx.len(), idx.mode_name()),
            Err(e) => println!("✗ bozuk: {e}"),
        }
    } else {
        println!("yok (ilk kullanımda otomatik oluşturulur)");
    }
    Ok(())
}
