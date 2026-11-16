//! Agent loop: model → tool_calls → çalıştır → context'e ekle → tekrar.
//! Guardrail: max_iterations aşılınca araçsız son yanıt istenir.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Config;
use crate::llama::{LlamaClient, Message};
use crate::rag::Index;
use crate::tools::{self, ToolCtx};

pub const SYSTEM_PROMPT: &str = "Sen Lalasın: kullanıcısına refakat eden bilge ve ölçülü bir yerel asistansın. Türkçe konuş.\n\
Araçlar, kullanım önceliği sırasıyla:\n\
1. retrieve — kullanıcının yerel belgelerinde (korpus) arar. Bilgi gerektiren HER soruda (tanım, kavram, coğrafya, tarih, proje, belge) ÖNCE bunu kullan.\n\
2. web_search — yalnızca güncel/günlük bilgi için (fiyat, hava durumu, son dakika) ve korpusun kapsamadığından emin olduktan sonra.\n\
3. fetch_url — web_search'ten çıkan bir sayfayı derinlemesine okumak için.\n\
Kurallar:\n\
- retrieve sonuçlarını kendi cümlelerinle, korpus metnine sadık kalarak özetle; korpusun söylemediği hiçbir bilgiyi ekleme.\n\
- Bir araç hata verirse ASLA bilgi uydurma; başka bir aracı deneyebilirsin, o da yoksa bilginin şu an erişilebilir olmadığını açıkça söyle.\n\
- Kullandığın kaynakları metin içinde [1], [2] biçiminde işaretle.\n\
- Araç gerektirmeyen durumlarda (selamlaşma, kısa sohbet) doğrudan yanıtla.";

pub async fn run_turn(
    cfg: &Config,
    client: &LlamaClient,
    index: &Index,
    messages: &mut Vec<Message>,
    max_tokens: u32,
) -> Result<(String, Vec<String>)> {
    let defs = tools::definitions();
    let ctx = &ToolCtx {
        http: client.http(),
        index,
        searxng_url: &cfg.searxng_url,
        top_k: cfg.top_k,
        embed: Some(client),
    };
    let mut sources: Vec<String> = Vec::new();
    let mut iterations = 0usize;
    loop {
        iterations += 1;
        let use_tools = iterations <= cfg.max_iterations;
        let (msg, _usage) = client
            .chat(
                messages,
                if use_tools { Some(&defs) } else { None },
                None,
                max_tokens,
            )
            .await
            .context("model çağrısı başarısız")?;
        let calls = msg.tool_calls.clone();
        if use_tools {
            if let Some(calls) = calls {
                messages.push(msg);
                // Paralel araç çalıştırma: aynı turdaki çağrılar birlikte koşar,
                // sonuçlar orijinal çağrı sırasıyla geri beslenir.
                let jobs = calls.into_iter().map(|c| {
                    let name = c.function.name.clone();
                    let args: Value = serde_json::from_str(&c.function.arguments)
                        .unwrap_or(Value::Null);
                    async move {
                        let res = tools::execute(&name, &args, &ctx).await;
                        let tag = if res.is_ok() { "ok" } else { "hata" };
                        println!("  [araç] {name} → {tag}");
                        let mut content = match &res {
                            Ok(s) => s.clone(),
                            Err(e) => format!(
                                "HATA: {e:#} Bu araç şu an erişilemez. ASLA bilgi uydurma: ya retrieve aracını dene ya da bu bilgiye şu an erişemediğini açıkça söyle."
                            ),
                        };
                        let mut fb_sources: Vec<String> = Vec::new();
                        // Guardrail: web_search düşerse döngü otomatik retrieve dener —
                        // modelin araç seçim disiplinine güvenilmez.
                        if name == "web_search" && res.is_err() {
                            let fq = args
                                .get("query")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !fq.is_empty() {
                                println!("  [araç] retrieve (otomatik yedek)");
                                match tools::execute(
                                    "retrieve",
                                    &serde_json::json!({ "query": fq }),
                                    &ctx,
                                )
                                .await
                                {
                                    Ok(s) => {
                                        collect_sources("retrieve", &s, &mut fb_sources);
                                        content.push_str(
                                            "\n\n[sistem yedeği] korpus araması otomatik yapıldı:\n",
                                        );
                                        content.push_str(&s);
                                    }
                                    Err(e) => content.push_str(&format!(
                                        "\n\n[sistem yedeği] korpus araması da başarısız: {e:#}"
                                    )),
                                }
                            }
                        }
                        (c.id, name, res, content, fb_sources)
                    }
                });
                let done = futures::future::join_all(jobs).await;
                for (id, name, res, content, fb_sources) in done {
                    if let Ok(s) = &res {
                        collect_sources(&name, s, &mut sources);
                    }
                    for s in fb_sources {
                        push_unique(&mut sources, s);
                    }
                    messages.push(Message::tool_result(id, content));
                }
                continue;
            }
        }
        let mut answer = msg.content.unwrap_or_default();
        // Guardrail: küçük model düşünmeye tüm token'i harcayıp boş yanıt
        // döndürebilir; deterministik dürüst mesaj ver.
        if answer.trim().is_empty() {
            answer = if sources.is_empty() {
                "Üzgünüm, bu soruya şu an yanıt bulamadım: erişilebilir bir kaynak olmadı.".to_string()
            } else {
                "Üzgünüm, bulduğum kaynaklardan tutarlı bir yanıt üretemedim; sorunuzu farklı ifade etmeyi dener misin?".to_string()
            };
        }
        return Ok((answer, sources));
    }
}

fn collect_sources(tool: &str, result: &str, out: &mut Vec<String>) {
    let Ok(v) = serde_json::from_str::<Value>(result) else {
        return;
    };
    match tool {
        "retrieve" => {
            if let Some(items) = v.get("sonuclar").and_then(|x| x.as_array()) {
                for it in items {
                    if let Some(s) = it.get("kaynak").and_then(|x| x.as_str()) {
                        push_unique(out, format!("belge:{s}"));
                    }
                }
            }
        }
        "web_search" => {
            if let Some(items) = v.get("sonuclar").and_then(|x| x.as_array()) {
                for it in items {
                    if let Some(s) = it.get("url").and_then(|x| x.as_str()) {
                        push_unique(out, s.to_string());
                    }
                }
            }
        }
        "fetch_url" => {
            if let Some(s) = v.get("url").and_then(|x| x.as_str()) {
                push_unique(out, s.to_string());
            }
        }
        _ => {}
    }
}

fn push_unique(out: &mut Vec<String>, s: String) {
    if !out.contains(&s) {
        out.push(s);
    }
}
