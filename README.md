# Lala

> Osmanlı sarayında *lala*, şehzadenin eğitimini üstlenen bilge refakatçidir.
> Bu projede ise az parametreli, tamamen yerel çalışan bir dil modeli aynı
> rolü oynar: büyük bir zeka değil, doğru zamanda doğru araca uzanan sakin
> bir yol gösterici.

Lala, llama.cpp üzerinde koşan açık ağırlıklı küçük bir SLM'e Rust ile
yazılmış bir agent loop katan minimal ajan iskeletidir. Her şey sizin
makinenizde koşar: model, arama motoru, belge korpusu. Anahtar yok, kota
yok, bulut yok.

> **Model önerisi (test edilmiştir)**: sohbet için **Qwen3-1.7B Q4_K_M**
> önerilir — 0.6B doğru belgeyi bulur ama sentez savunaklıdır; 1.7B üç
> testte de akıcı, sadık ve dürüst davrandı. 0.6B minimum boyut olarak
> çalışır; 16 GB VRAM ikisini birden rahat kaldırır.

```
┌────────────┐  tool_calls   ┌──────────────────────────────┐
│  Qwen3     │◄─────────────►│  lala (Rust agent loop)      │
│  GGUF      │   JSON        │  - guardrail'ler             │
└────────────┘               │  - kaynak toplama            │
   llama-server              │  - KV-cache dostu önek       │
   :8123 (Vulkan)            └──────┬───────┬───────┬───────┘
                                    │       │       │
                              retrieve  web_search fetch_url
                                    │       │       │
                              .lala-index  SearXNG  http
                             (Qwen3-Embedding  :8888  html→metin
                              :8125 | BM25)
```

## Özellikler

- **Üç araç**: `retrieve` (yerel RAG), `web_search` (SearXNG), `fetch_url`
  (sayfa → metin). Araç şemaları sabit sırada üretilir; llama.cpp'nin
  önek KV-cache'i böylece her turda dolu dolu kullanılır.
- **Paralel araç yürütme**: aynı turdaki birden çok araç çağrısı birlikte
  koşar, sonuçlar çağrı sırasıyla geri beslenir.
- **Döngü seviyesinde guardrail'ler**: azami araç turu, araç hata mesajına
  gömülü anti-uydurma yönergesi, web_search düşerse otomatik retrieve
  yedeği, boş yanıtta dürüst mesaj. Model disiplinine güvenilmez —
  bunlar deterministik Rust kodudur.
- **RAG, çift modlu**: Qwen3-Embedding ile anlamsal arama (embed modu);
  embedding sunucusu yoksa BM25'e düşer (lexical mod) ve çevrimdışı
  çalışır. Türkçe eklemeli morfoloji için ön-ek eşleşmesi içerir
  ("başkent" ↔ "başkentidir").
- **Grammar-constrained decoding**: `lala extract` çıktıyı JSON şemaya
  hapseder — geçerlilik dilbilgisi düzeyinde garanti edilir, retry yok.
- **Kaynak atıfları**: cevapta [1], [2] işaretleri + altta kaynak listesi.

## Kurulum

### Gereksinimler

- Rust (stable)
- [llama.cpp](https://github.com/ggml-org/llama.cpp) — Windows'ta
  release sayfasındaki `bin-win-vulkan-x64` paketi AMD kartlarda
  CUDA'sız çalışır (herhangi bir backend de olur)
- Modeller (GGUF): sohbet için `Qwen3-0.6B-Q8_0` (veya `Q4_K_M`),
  RAG için `Qwen/Qwen3-Embedding-0.6B-GGUF`
- (opsiyonel, web_search için) Docker + SearXNG

### Sunucular

```bash
# Sohbet modeli
llama-server -m Qwen3-0.6B-Q8_0.gguf -ngl 99 -c 16384 --jinja --port 8123

# Embedding modeli (RAG için; kapalıysa lala BM25'e düşer)
llama-server -m Qwen3-Embedding-0.6B-Q8_0.gguf --embedding -c 8192 -ngl 99 --port 8125

# SearXNG (web_search için; kapalıysa web aramaları dürüstçe hata verir)
cd deploy && docker compose up -d
# Not: deploy/searxng/settings.yml içinde secret_key'i değiştirin.
```

