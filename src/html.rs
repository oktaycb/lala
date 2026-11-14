//! Minimal HTML → metin dönüştürücü (bağımlılıksız).
//! fetch_url aracının sayfa içeriğini okunur metne indirger.

pub fn strip_html(input: &str) -> String {
    #[derive(PartialEq)]
    enum St {
        Text,
        Tag,
        Script,
        Style,
    }

    let mut st = St::Text;
    let mut buf = String::with_capacity(input.len() / 3 + 16);
    let mut tag = String::new();
    let mut watch = String::new();

    for c in input.chars() {
        match st {
            St::Text => {
                if c == '<' {
                    tag.clear();
                    st = St::Tag;
                    buf.push(' ');
                } else {
                    buf.push(c);
                }
            }
            St::Tag => {
                if c == '>' {
                    let t = tag.trim().to_lowercase();
                    let first = t
                        .split(|ch: char| ch.is_whitespace())
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if first == "script" {
                        watch.clear();
                        st = St::Script;
                    } else if first == "style" {
                        watch.clear();
                        st = St::Style;
                    } else {
                        st = St::Text;
                    }
                } else {
                    tag.push(c);
                }
            }
            St::Script | St::Style => {
                watch.push(c);
                if watch.len() > 12 {
                    watch.remove(0);
                }
                let closer = if st == St::Script { "</script>" } else { "</style>" };
                if watch.to_lowercase().ends_with(closer) {
                    watch.clear();
                    st = St::Text;
                }
            }
        }
    }

    let out: String = buf
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    let mut collapsed = String::with_capacity(out.len());
    let mut last_space = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !last_space {
                collapsed.push(' ');
            }
            last_space = true;
        } else {
            collapsed.push(ch);
            last_space = false;
        }
    }
    collapsed.trim().to_string()
}

