# RAG nedir?

RAG (Retrieval-Augmented Generation), modelin cevabını kendi bildikleriyle
sınırlamayıp cevap üretmeden önce ilgili belgeleri geri çağırması
(retrieval) fikridir. Korpus parçalara bölünür, her parça bir embedding
vektörüne çevrilir; soru geldiğinde sorunun vektörü ile parçaların
vektörleri karşılaştırılıp en benzer birkaç parça bağlama eklenir.

Lala'da retrieve aracı iki modda çalışır. Embedding modunda Qwen3-Embedding
gibi çok dilli bir gömme modeli kullanılır ve Türkçe sorularla Türkçe
belgeler arasında anlam düzeyinde eşleşme bulunur. Embedding sunucusu kapalıysa
lexical (BM25) moduna düşer: kelime temelli istatistiksel skorlama, model
gerekmez, çevrimdışı çalışır.

İyi bir RAG'ın sırrı parçalamadadır: çok küçük parçalar bağlamı koparır,
çok büyük parçalar pencereyi doldurur. Lala paragraf sınırlarına saygılı,
hedefi yaklaşık bin karakterlik parçalar üretir.
