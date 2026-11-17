//! lala-gen — SFT senaryolarını (sft/scenarios.jsonl) eğitim örneklerine
//! çevirir (sft/train.jsonl).
//!
//! Çıktı biçimi: her satır {"messages": [...], "tools": [...]} — Qwen3
//! sohbet şablonu ve TRL ile uyumlu; araç şemaları çalışma zamanıyla birebir
//! aynıdır (lala::tools::definitions), sistem prompt'u agent::SYSTEM_PROMPT'tur.
//!
//! Kullanım: cargo run --bin lala-gen

use anyhow::{bail, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};

#[derive(Parser)]
struct Args {
    /// Senaryo dosyası (JSONL)
    #[arg(long, default_value = "sft/scenarios.jsonl")]
    scenarios: String,
    /// Çıktı eğitim dosyası (JSONL)
    #[arg(long, default_value = "sft/train.jsonl")]
    out: String,
}

#[derive(Deserialize)]
struct Scenario {
    id: String,
    #[serde(default)]
    kategori: String,
    user: String,
    #[serde(default)]
    adimlar: Vec<Adim>,
    #[serde(rename = "final")]
    final_cevap: String,
    #[serde(default)]
    kaynaklar: Vec<String>,
}

#[derive(Deserialize)]
struct Adim {
    arac: String,
    args: serde_json::Value,
    /// Aracın döndürdüğü içerik (başarılıysa JSON dizgesi; hata adımlarında serbest metin)
    sonuc: String,
    /// true ise adım bir araç hatasını temsil eder (sonuc JSON olmak zorunda değil)
    #[serde(default)]
    #[allow(dead_code)]
    hata: bool,
}

#[derive(Serialize)]
struct TrainExample {
    messages: Vec<serde_json::Value>,
    tools: serde_json::Value,
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("lala-gen hatası: {e:#}");
        std::process::exit(1);
    }
}

