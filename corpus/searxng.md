# SearXNG nedir?

SearXNG, açık kaynaklı meta bir arama motorudur: Google, Bing, DuckDuckGo
gibi arama motorlarını arkada topluca sorgular, sonuçları birleştirir ve
kullanıcıyı takip etmez. Kendi makinenizde docker ile tek komutta
çalıştırılabilir; API anahtarı, kota ve fatura derdi yoktur.

Lala'nın web_search aracı SearXNG'nin JSON biçimli arama ucunu kullanır.
Self-hosted olması projenin ruhuna uyar: ajanın tüm bilgi kaynağı, tıpkı
model ve korpus gibi, kullanıcının kontrolünde kalır.

Kurulum tipik olarak şöyledir: docker run -p 8888:8080 searxng/searxng
ile başlatılır; JSON formatının izinli olması için settings.yml içinde
search.formats listesine json eklenir. Lala varsayılan olarak 8888
portundaki örneğe bakar; LALA_SEARXNG_URL ortam değişkeniyle değiştirilir.
