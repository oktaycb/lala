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

### Lala

```bash
cargo build --release
./target/release/lala doctor     # sunucu/korpus/içindekiler durumu
```

### Yapılandırma

| Değişken | Varsayılan | Anlamı |
|---|---|---|
| `LALA_CHAT_URL` | `http://127.0.0.1:8123` | sohbet modeli sunucusu |
| `LALA_EMBED_URL` | `http://127.0.0.1:8125` | embedding sunucusu |
| `LALA_SEARXNG_URL` | `http://127.0.0.1:8888` | SearXNG örneği |
| `LALA_CORPUS` | `./corpus` | belge dizini (`.md`, `.txt`) |

## Kullanım

```bash
lala index                          # korpusu indeksle (vektörler cache'lenir)
lala ask "Korpusa göre lala nedir?" # tek seferlik soru
lala chat                           # etkileşimli REPL (/yardim, /araclar, /sifirla, /cik)
lala extract "metin..." --schema examples/ozet-schema.json   # şemaya hapsolmuş JSON
lala doctor                         # tüm bileşenlerin sağlık durumu
```

`corpus/` dizinine kendi `.md`/`.txt` dosyalarınızı atıp `lala index`
koşturmanız yeterli; paragraf sınırlarına saygılı ~1000 karakterlik
parçalara bölünüp gömülür. Örnek korpus (projenin kendi kavramları ve
Türkiye notları) dahil gelmektedir.

## SFT veri hattı (sonraki faz için hazır)

`sft/` dizininde sentetik tool-call dataset iskeleti duruyor: el yazması
senaryolar (`scenarios.jsonl`, 31 adet) → `cargo run --bin lala-gen` →
TRL/Qwen3 şablonuyla uyumlu eğitim örnekleri. Üreteç, sistem prompt'unu ve
araç şemalarını **çalışma zamanıyla aynı kaynaktan** alır (lib crate) —
eğitim/inference dağılım kayması yok. Detaylar: [sft/FORMAT.md](sft/FORMAT.md).

## Mimari kararlar

- **Model disiplinine değil döngüye güven**: 0.6B'lik modelde araç seçimi
  ve hata sonrası davranış dalgalıdır. Lala bu belirsizliği modelden
  Rust'a taşır: hatadan sonra uydurma yasağı araç sonucunun içine gömülür,
  web_search düşerse döngü bizzat retrieve koşturur, boş cevaba dürüst
  mesaj konur.
- **Düşük sıcaklık (0.4)**: küçük modellerde araç çağrılarının biçimsel
  tutarlılığı için.
- **Sabit araç şeması sırası**: sistem mesajı + şemalar her istekte aynı
  öneki oluşturur; llama.cpp önek cache'i sayesinde prompt işleme
  10 kat hızlanabilir.
- **İki modlu RAG**: embedding sunucusu bir lüks; BM25 yedeği sayesinde
  lala tamamen çevrimdışı da çalışır.

## Bilinen sınırlar

- 0.6B'nin **sentez kalitesi** düşüktür: doğru belgeyi bulur, cümlelemede
 savunaklı olabilir. Yol haritasının ilk maddesi tam olarak bunun için:
  alan-özel LoRA SFT. Boyut merdiveni (1.7B, 4B) de aynı iskeletle çalışır.
- BM25 modu Türkçe morfolojisini yalnızca ön-ek eşleşmesiyle idare eder.
- `web_search` için SearXNG'nin `format=json` izni açık olmalıdır.

## Yol haritası

- [x] Sentetik tool-call dataset iskeleti (`sft/` + `lala-gen`)
- [x] Paralel araç çalıştırma
- [x] SearXNG docker-compose paketi (`deploy/`)
- [x] Boyut merdiveni ilk ölçüm: 1.7B Q4_K_M sentez/dürüstlükte 0.6B'yi
  belirgin şekilde geçti → önerilen varsayılan
- [ ] LoRA SFT (dataset büyütme → TRL+PEFT, tek GPU) — sentez kalitesinin
  kalıcı çözümü
- [ ] Qwen3-4B profili (merdivenin üst basamağı)
- [ ] Doğrudan GBNF grammar parametresi ile araç çağrısı kısıtlama opsiyonu

## Lisans

MIT — bkz. [LICENSE](LICENSE).
