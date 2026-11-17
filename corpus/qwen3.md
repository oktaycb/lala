# Qwen3 ailesi hakkında notlar

Qwen3, Alibaba'nın açık ağırlıklı dil modeli ailesidir; 0.6B'den 32B'ye
kadar boyları vardır ve aile 119 dil iddia eder, Türkçe de bu listenin
içindedir. Küçük boylar (0.6B, 1.7B) tek tük VRAM ile çalışır ve yerel
kullanım için uygundur.

Qwen3'ün iki modu vardır: düşünme (thinking) ve düşünmeyen (no_think).
Küçük boyutta düşünme modu belirleyicidir; basit bilgi soruları düşünme
kapalıyken kaçırılabilir, açıkken doğru cevaplanır. Araç çağrısı da ailenin
doğal yeteneğidir; model araç şemasını bağlamda görünce geçerli JSON
çağrıları üretir.

Qwen3-Embedding, aynı ailenin gömme (embedding) varyantıdır; belgeler ve
sorgular için ayrı formatlar kullanır. Türkçe dahil çok dilli aramada
kullanılabilir ve RAG retrieve aracının embedding modunun temelini
oluşturur.
