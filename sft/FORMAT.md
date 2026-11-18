# SFT veri hattı (scaffold)

Bu dizin, Hat B'nin bir sonraki fazı için hazır bekleyen **sentetik tool-call
SFT dataset'i** iskeletidir.

## Dosyalar

- `scenarios.jsonl` — el yazması senaryolar: her satır bir etkileşim (id,
  kategori, user, adımlar [araç + args + beklenen sonuç + `hata` bayrağı],
  final cevap, kaynaklar). Şu an 31 senaryo: rag (15), sohbet (4), web (3),
  zincir (2), hata-yedek (2), ret (2), paralel (2), fetch (1).
- `train.jsonl` — `lala-gen` çıktısı: TRL/Qwen3 şablonuyla uyumlu
  `{"messages": [...], "tools": [...]}` satırları. Git'e giren
  senaryolardır; train.jsonl yeniden üretilebilir (`cargo run --bin lala-gen`).

## Neden bu yapı?

- **Çalışma zamanıyla birebir uyum**: sistem prompt'u ve araç şemaları
  `lala::agent::SYSTEM_PROMPT` / `lala::tools::definitions`'ten gelir —
  inference'taki dağılımla eğitim dağılımı arasında sürüklenme olmaz.
- **Doğrulanmış üretim**: `lala-gen` her senaryoyu doğrular (araç adı,
  args zorunlu alanı, sonucun geçerli JSON'u, benzersiz id); bozuk satır
  build'i kırar.
- **Paralel gerçeklik**: aynı assistant turundaki çoklu araç çağrıları
  tek `tool_calls` bloğuna yazılır — döngünün paralel yürütmesine
  karşılık gelir.

## Öriteyi büyütmek (sonraki adım)

El yazması senaryolar tohumdur; ölçek için iki kanıtlanmış yol:

1. **Güçlü modelle sentez** (APIGen mantığı): büyük bir model (cloud API
   ya da lokal 7-8B) senaryo şablonlarından soru-iz-sonuç üçlülerini
   çoğaltır; insan onayından geçenler `scenarios.jsonl`'e katılır.
2. **Gerçek kayıt**: `lala chat` oturumlarını (opt-in ile) kaydedip
   temizlenen iyi örnekleri senaryoya çevirmek.

Hedef: kategori başına 100+ senaryo → birkaç bin varyasyon → tek GPU'da
saatler süren QLoRA.

## Eğitim tarafı (Hat B planı)

PyTorch ROCm (Linux/WSL2) veya bulut CUDA; TRL SFTTrainer + PEFT LoRA,
Qwen3 şablonu; sonra merge → GGUF → llama.cpp. Detaylar CLAUDE.md'de.
