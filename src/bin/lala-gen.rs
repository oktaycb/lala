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

fn run(args: Args) -> Result<()> {
    let file = File::open(&args.scenarios)
        .map_err(|e| anyhow::anyhow!("senaryo dosyası açılamadı ({}): {e}", args.scenarios))?;
    let reader = std::io::BufReader::new(file);
    let out = BufWriter::new(File::create(&args.out)?);

    let tools_schema = lala::tools::definitions();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut n_ok = 0usize;
    let mut kategoriler: std::collections::BTreeMap<String, usize> = Default::default();
    let mut arac_sayim: std::collections::BTreeMap<String, usize> = Default::default();

    let mut w = out;
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let sc: Scenario = serde_json::from_str(line)
            .map_err(|e| anyhow::anyhow!("satır {}: JSON bozuk: {e}", i + 1))?;

        // Doğrulama
        if !seen_ids.insert(sc.id.clone()) {
            bail!("satır {}: id tekrar ediyor: {}", i + 1, sc.id);
        }
        if sc.user.trim().is_empty() {
            bail!("satır {}: user boş", i + 1);
        }
        if sc.final_cevap.trim().is_empty() {
            bail!("satır {}: final boş", i + 1);
        }
        for adim in &sc.adimlar {
            if !matches!(adim.arac.as_str(), "retrieve" | "web_search" | "fetch_url") {
                bail!("satır {}: bilinmeyen araç: {}", i + 1, adim.arac);
            }
            if !adim.args.is_object() {
                bail!("satır {}: args nesne olmalı", i + 1);
            }
            let req = match adim.arac.as_str() {
                "fetch_url" => "url",
                _ => "query",
            };
            if adim.args.get(req).and_then(|v| v.as_str()).is_none() {
                bail!("satır {}: {} için '{}' eksik", i + 1, adim.arac, req);
            }
            if !adim.hata {
                serde_json::from_str::<serde_json::Value>(&adim.sonuc).map_err(|e| {
                    anyhow::anyhow!("satır {}: sonuc JSON değil: {e}", i + 1)
                })?;
            }
            *arac_sayim.entry(adim.arac.clone()).or_insert(0) += 1;
        }

        // Mesaj dizisini kur
        let mut messages = vec![
            json!({"role": "system", "content": lala::agent::SYSTEM_PROMPT}),
            json!({"role": "user", "content": sc.user}),
        ];
        // Tek assistant turunda tüm araç çağrıları (çalışma zamanındaki
        // paralel yürütmeye karşılık gelir), ardından sırayla tool mesajları.
        if !sc.adimlar.is_empty() {
            let tool_calls: Vec<serde_json::Value> = sc
                .adimlar
                .iter()
                .enumerate()
                .map(|(j, a)| {
                    json!({
                        "id": format!("call_{}", j + 1),
                        "type": "function",
                        "function": {
                            "name": a.arac,
                            "arguments": serde_json::to_string(&a.args).unwrap_or_default(),
                        }
                    })
                })
                .collect();
            messages.push(json!({
                "role": "assistant",
                "content": "",
                "tool_calls": tool_calls,
            }));
            for (j, a) in sc.adimlar.iter().enumerate() {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": format!("call_{}", j + 1),
                    "name": a.arac,
                    "content": a.sonuc,
                }));
            }
        }
        messages.push(json!({"role": "assistant", "content": sc.final_cevap}));

        let ex = TrainExample {
            messages,
            tools: tools_schema.clone(),
        };
        writeln!(w, "{}", serde_json::to_string(&ex)?)?;
        n_ok += 1;
        *kategoriler.entry(sc.kategori.clone()).or_insert(0) += 1;
    }
    w.flush()?;

    println!("✓ {n_ok} örnek → {}", args.out);
    println!("  kategoriler: {kategoriler:?}");
    println!("  araç kullanımları: {arac_sayim:?}");
    if let Some(k) = kategoriler.get("") {
        println!(
            "  [!] {k} senaryonun kategorisi boş — sft/FORMAT.md'ye bakın"
        );
    }
    Ok(())
}
