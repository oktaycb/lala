# Agent loop nedir?

Ajan döngüsü, dil modelinin tek seferlik cevap makinesi olmaktan çıkıp çok
adımlı iş yapmasının yoludur. Model bir mesaj alır; cevap yerine araç
çağrısı üretirse döngü bu çağrıyı çalıştırır, sonucu sohbet geçmişine
"tool" rolünde ekler ve modeli yeniden çağırır. Model bu kez sonucu görüp
ya yeni bir araç çağrısı ya da nihai cevabı üretir.

Sağlıklı bir döngünün üç guardrail'i vardır: azami tur sayısı (sonsuz araç
çağrısı döngüsünü keser), araç başına zaman aşımı ve sonuçların
budanması. Budama kritiktir; arama sonuçlarını ham hâlde bağlama dökerseniz
küçük modellerin sınırlı bağlam penceresi hızla dolar.

Lala'nın döngüsünde azami altı araç turu vardır; her araç sonucu sıkıştırılmış
JSON olarak geri beslenir ve kullanılan kaynaklar cevabın altında listelenir.
Ayrıca sohbet öneki (sistem mesajı + araç şemaları) her turda aynı sırada
tutulur; llama.cpp bu öneki önbelleğinden yararlanarak sonraki turlarda
prompt işlemeyi çok hızlandırır.
