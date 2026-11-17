# llama.cpp notları

llama.cpp, GGUF biçimindeki model dosyalarını CPU ve GPU'da koşturan, C++
ile yazılmış çıplak-metal çıkarım motorudur. llm araç evreninin taş duvarı
gibi yapısal parçasıdır: OpenAI uyumlu bir HTTP sunucusu (llama-server),
GBNF dilbilgisi kısıtlaması, KV önbelleği ve Vulkan, CUDA, Metal gibi çok
sayıda backend ile gelir.

AMD kartlı makinelerde Vulkan backend'i en pratik yoldur; CUDA gerekmez.
16 GB VRAM'li bir kartta 0.6B'lik model tam yüklemede saniyede yüzlerce
token üretir; tek kullanıcı için interaktif kullanımın çok ötesindedir.

llama-server'ın /v1/chat/completions ucu araç çağrılarını da anlar;
--jinja bayrağıyla modelin kendi sohbet şablonu devreye girer.
response_format içinde verilen json_schema, üretimi şemaya hapseder ve
çıktının geçerli JSON olmasını dilbilgisi düzeyinde garanti eder.
KV önbelleği önek tabanlıdır: aynı sistem mesajı ve araç şemaları her
turda aynı sırada gönderilirse sonraki isteklerde prompt işleme neredeyse
bedavaya düşer.
