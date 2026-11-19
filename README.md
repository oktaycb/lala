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

