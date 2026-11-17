# Türkçe token ekonomisi

Dil modelleri metni token diye bölümlere ayırarak okur; tokenizer'ın bir
dili ne kadar iyi kapsadığı, aynı metnin kaç token'a mal olacağını belirler.
İngilizce için tipik sıkışma dört karaktere bir token civarındayken,
kapsaması zayıf dillerde bu oran ikiye kadar düşebilir; yani aynı anlam
pencereyi iki kat hızlı doldurur.

Qwen3'ün tokenizer'ı Türkçe için gayet makuldür: ölçümler yaklaşık 3.5
karaktere bir token gösterir; İngilizceye göre şişme yüzde yirmi beş
civarındadır. Bu yüzden yerel projelerde ayrıca özel bir Türkçe tokenizer
eğitmek çoğu zaman gereksizdir; kazanılacak alan küçüktür.

Türkçe morfolojisi eklemeli olduğu için tek kelime çok ek alabilir;
ekli biçimler (örneğin "öğrencilerimizden") tokenizer'da birden çok
parçaya ayrılır. Bu, jenerik tokenleştiricilerin doğal sınırıdır ve
alan-özel modellerde ancak eğitim verisiyle hafifletilir.
