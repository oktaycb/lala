# Lala nedir?

Lala, kullanıcının kendi bilgisayarında çalışan küçük bir dil modeline araç
kullanma yeteneği katan yerel bir ajan iskeletidir. Osmanlı sarayında lala,
şehzadenin eğitimini üstlenen bilge refakatçi demektir; bu projedeki küçük
model de tam olarak bu rolü oynar: büyük bir zeka değildir, ama doğru
zamanla doğru araca uzanan sakin bir yol göstericidir.

Lala'nın beyni llama.cpp üzerinden koşan Qwen3 gibi açık ağırlıklı küçük bir
modeldir. Sinir sistemi ise Rust ile yazılmış ajan döngüsüdür: model bir araç
çağrısı üretir, döngü bu çağrıyı çalıştırır, sonucu bağlama ekler ve model
tekrar konuşur. Döngü üstünlüğü buradadır: Python'dan milisaniyelerle değil,
ama geçerli JSON garantisi ve paralel araç çalıştırmayla kazanılır.

Üç araçla doğar: retrieve yerel belgelerde arar, web_search SearXNG
üzerinden internette arar, fetch_url bir sayfanın metnini çeker. Grammar
constrained decoding sayesinde modelin ürettiği araç çağrıları biçimsel
olarak her zaman geçerlidir; anlamı ise modelin ve eğitim verisinin işidir.
