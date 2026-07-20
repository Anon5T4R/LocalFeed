//! Cache local das imagens dos artigos.
//!
//! **O motivo não é velocidade, é privacidade.** Sem isto, abrir um artigo
//! dispara uma requisição por imagem pro servidor do site — que vê IP, horário
//! e exatamente o que você leu. Num leitor que se vende como local-first isso é
//! um vazamento por construção, e o usuário não tem como perceber.
//!
//! Como funciona: no primeiro acesso, cada imagem é baixada UMA vez e guardada
//! em `app_data/img-cache/<sha256 da url>`. Na hora de exibir, o `<img src>`
//! remoto é trocado por um `data:` com os bytes do disco — o webview nunca sai
//! pra rede, nem na primeira abertura (o download é nosso, com o nosso
//! user-agent, sem cookie e sem referer).
//!
//! Falha de rede não quebra nada: a imagem que não baixou fica com a URL
//! original, e a próxima abertura tenta de novo.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Teto por imagem. Acima disso a URL fica remota: guardar (e depois embutir em
/// base64, que cresce 33%) um arquivo de dezenas de MB pra ilustrar um texto
/// custa mais do que o problema que resolve.
const MAX_IMG_BYTES: usize = 5 * 1024 * 1024;
/// Teto por artigo. Galeria de 200 fotos não vira 200 requisições em rajada.
const MAX_IMGS: usize = 40;

pub fn cache_dir(app_data: &Path) -> PathBuf {
    app_data.join("img-cache")
}

fn key(url: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    format!("{:x}", h.finalize())
}

/// Extrai as URLs de `<img src="...">` do HTML, na ordem em que aparecem e sem
/// repetir. Só http(s): `data:` já está embutido e caminho relativo não dá pra
/// resolver sem a base (o readability já absolutiza o que consegue).
pub fn img_urls(html: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while let Some(p) = find_ci(bytes, i, b"<img") {
        let end = find_ci(bytes, p, b">").unwrap_or(bytes.len());
        let tag = &html[p..end.min(html.len())];
        if let Some(u) = attr_value(tag, "src") {
            if (u.starts_with("http://") || u.starts_with("https://")) && !out.contains(&u) {
                out.push(u);
            }
        }
        i = end.max(p + 4);
    }
    out
}

fn find_ci(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from >= hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
        .map(|p| p + from)
}

/// Valor de um atributo dentro de uma tag, com aspas duplas ou simples.
fn attr_value(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(p) = lower[from..].find(name) {
        let at = from + p;
        // Tem que ser um atributo inteiro, não o sufixo de outro (`data-src`
        // casaria com `src` e devolveria o valor errado).
        let antes_ok = at == 0
            || lower.as_bytes()[at - 1] == b' '
            || lower.as_bytes()[at - 1] == b'\t'
            || lower.as_bytes()[at - 1] == b'\n';
        let resto = &lower[at + name.len()..];
        let depois = resto.trim_start();
        if antes_ok && depois.starts_with('=') {
            let eq = at + name.len() + (resto.len() - depois.len()) + 1;
            let v = tag[eq..].trim_start();
            let q = v.chars().next()?;
            if q == '"' || q == '\'' {
                let fim = v[1..].find(q)? + 1;
                return Some(v[1..fim].trim().to_string());
            }
            let fim = v.find([' ', '\t', '\n', '>']).unwrap_or(v.len());
            return Some(v[..fim].trim().to_string());
        }
        from = at + name.len();
    }
    None
}

/// Baixa pro cache o que ainda não está lá. Não devolve erro: imagem é
/// enfeite, e um artigo sem foto é melhor que um artigo que não abre.
pub fn warm(client: &reqwest::blocking::Client, dir: &Path, urls: &[String]) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    for url in urls.iter().take(MAX_IMGS) {
        let path = dir.join(key(url));
        if path.exists() {
            continue;
        }
        let Ok(resp) = client.get(url).send() else { continue };
        if !resp.status().is_success() {
            continue;
        }
        // Content-Length é dica, não garantia — o corte de verdade é no
        // tamanho lido. A dica só evita começar um download inútil.
        if resp.content_length().is_some_and(|n| n as usize > MAX_IMG_BYTES) {
            continue;
        }
        let mime = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !mime.starts_with("image/") {
            continue;
        }
        let Ok(bytes) = resp.bytes() else { continue };
        if bytes.len() > MAX_IMG_BYTES || bytes.is_empty() {
            continue;
        }
        // O mime vai junto no arquivo (primeira linha) porque o `data:`
        // precisa dele e adivinhar por extensão de URL erra em CDN.
        let mut buf = Vec::with_capacity(bytes.len() + mime.len() + 1);
        buf.extend_from_slice(mime.as_bytes());
        buf.push(b'\n');
        buf.extend_from_slice(&bytes);
        let _ = std::fs::write(path, buf);
    }
}

/// Lê o que está no cache e devolve `url -> data:` pronto pro src.
pub fn data_uris(dir: &Path, urls: &[String]) -> HashMap<String, String> {
    use base64::Engine;
    let mut map = HashMap::new();
    for url in urls {
        let Ok(raw) = std::fs::read(dir.join(key(url))) else { continue };
        let Some(nl) = raw.iter().position(|b| *b == b'\n') else { continue };
        let Ok(mime) = std::str::from_utf8(&raw[..nl]) else { continue };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&raw[nl + 1..]);
        map.insert(url.clone(), format!("data:{mime};base64,{b64}"));
    }
    map
}

/// Troca no HTML os `src` que estão no cache pelos `data:` correspondentes.
///
/// Tira `srcset` e `sizes` de TODA imagem, cacheada ou não: o navegador prefere
/// o `srcset` quando ele existe, então reescrever só o `src` deixaria a
/// requisição remota acontecer do mesmo jeito — o cache pareceria funcionar e
/// o vazamento continuaria.
pub fn rewrite(html: &str, map: &HashMap<String, String>) -> String {
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while let Some(p) = find_ci(bytes, i, b"<img") {
        let end = find_ci(bytes, p, b">").map(|e| e + 1).unwrap_or(bytes.len());
        out.push_str(&html[i..p]);
        let tag = &html[p..end.min(html.len())];
        out.push_str(&rewrite_tag(tag, map));
        i = end;
    }
    out.push_str(&html[i.min(html.len())..]);
    out
}

fn rewrite_tag(tag: &str, map: &HashMap<String, String>) -> String {
    let src = attr_value(tag, "src");
    let mut novo = strip_attr(tag, "srcset");
    novo = strip_attr(&novo, "sizes");
    if let Some(data) = src.as_ref().and_then(|u| map.get(u)) {
        novo = replace_attr(&novo, "src", data);
    }
    novo
}

fn strip_attr(tag: &str, name: &str) -> String {
    match attr_span(tag, name) {
        Some((a, b)) => format!("{}{}", &tag[..a], &tag[b..]),
        None => tag.to_string(),
    }
}

fn replace_attr(tag: &str, name: &str, value: &str) -> String {
    match attr_span(tag, name) {
        Some((a, b)) => format!("{}{}=\"{}\"{}", &tag[..a], name, value, &tag[b..]),
        None => tag.to_string(),
    }
}

/// Intervalo `nome="valor"` dentro da tag (inclui o espaço anterior).
fn attr_span(tag: &str, name: &str) -> Option<(usize, usize)> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(p) = lower[from..].find(name) {
        let at = from + p;
        let antes_ok = at > 0 && matches!(lower.as_bytes()[at - 1], b' ' | b'\t' | b'\n');
        let resto = &lower[at + name.len()..];
        let depois = resto.trim_start();
        if antes_ok && depois.starts_with('=') {
            let eq = at + name.len() + (resto.len() - depois.len()) + 1;
            let v = &tag[eq..];
            let sem_esp = v.trim_start();
            let off = eq + (v.len() - sem_esp.len());
            let fim = match sem_esp.chars().next() {
                Some(q @ ('"' | '\'')) => off + 1 + sem_esp[1..].find(q)? + 1,
                _ => off + sem_esp.find([' ', '\t', '\n', '>']).unwrap_or(sem_esp.len()),
            };
            return Some((at.saturating_sub(1), fim));
        }
        from = at + name.len();
    }
    None
}

/// Bytes ocupados pelo cache (pro painel "Dados e armazenamento").
pub fn dir_bytes(dir: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    rd.filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Apaga o cache inteiro. Seguro: é 100% derivado — as imagens voltam a ser
/// baixadas na próxima abertura do artigo.
pub fn clear(dir: &Path) -> u64 {
    let n = dir_bytes(dir);
    let _ = std::fs::remove_dir_all(dir);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn img_urls_pega_so_http_sem_repetir() {
        let html = r#"
          <p><img src="https://a.com/1.png" alt="um"></p>
          <IMG SRC='http://b.com/2.jpg'>
          <img src="data:image/png;base64,AAAA">
          <img src="/relativo.png">
          <img src="https://a.com/1.png">
        "#;
        assert_eq!(
            img_urls(html),
            vec!["https://a.com/1.png".to_string(), "http://b.com/2.jpg".to_string()]
        );
    }

    #[test]
    fn data_src_nao_e_confundido_com_src() {
        // `data-src` termina em "src": um `find("src")` ingênuo devolveria o
        // lazy-load em vez da imagem real.
        let tag = r#"<img data-src="https://lazy.com/x.png" src="https://real.com/y.png">"#;
        assert_eq!(attr_value(tag, "src").as_deref(), Some("https://real.com/y.png"));
    }

    #[test]
    fn rewrite_troca_o_src_e_mata_o_srcset() {
        let mut map = HashMap::new();
        map.insert("https://a.com/1.png".to_string(), "data:image/png;base64,QQ==".to_string());
        let html = r#"<p>oi</p><img src="https://a.com/1.png" srcset="https://a.com/1@2x.png 2x" sizes="50vw" alt="x"><p>fim</p>"#;
        let out = rewrite(html, &map);
        assert!(out.contains(r#"src="data:image/png;base64,QQ==""#));
        // O srcset PRECISA sumir: se ficar, o navegador busca a versão 2x na
        // rede e o cache não serviu pra nada.
        assert!(!out.contains("srcset"));
        assert!(!out.contains("sizes"));
        // O que não era imagem fica intacto.
        assert!(out.contains("<p>oi</p>") && out.contains("<p>fim</p>"));
        assert!(out.contains(r#"alt="x""#));
    }

    #[test]
    fn imagem_fora_do_cache_mantem_a_url_original() {
        let map = HashMap::new();
        let html = r#"<img src="https://a.com/1.png">"#;
        assert_eq!(rewrite(html, &map), r#"<img src="https://a.com/1.png">"#);
    }


    /// Servidor HTTP mínimo num thread: `warm` só vale se o download de
    /// verdade for exercitado. Devolve (porta, quantas requisições recebeu).
    fn servidor(
        respostas: Vec<(&'static str, Vec<u8>)>,
    ) -> (u16, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let lis = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let porta = lis.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        std::thread::spawn(move || {
            for stream in lis.incoming().take(respostas.len() * 3) {
                let Ok(mut st) = stream else { break };
                let mut buf = [0u8; 1024];
                let n = st.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                h.fetch_add(1, Ordering::SeqCst);
                // O caminho é /0, /1, ... e escolhe qual resposta sai.
                let idx: usize = req
                    .split_whitespace()
                    .nth(1)
                    .and_then(|p| p.trim_start_matches('/').parse().ok())
                    .unwrap_or(0);
                let (mime, corpo) = &respostas[idx.min(respostas.len() - 1)];
                let cab = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    corpo.len()
                );
                let _ = st.write_all(cab.as_bytes());
                let _ = st.write_all(corpo);
                let _ = st.flush();
            }
        });
        (porta, hits)
    }

    #[test]
    fn warm_baixa_uma_vez_e_recusa_o_que_nao_e_imagem_ou_e_grande_demais() {
        use std::sync::atomic::Ordering;

        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
        let gigante = vec![0u8; MAX_IMG_BYTES + 1];
        let (porta, hits) = servidor(vec![
            ("image/png", png.clone()),
            ("text/html; charset=utf-8", b"<html>nao sou imagem</html>".to_vec()),
            ("image/jpeg", gigante),
        ]);

        let dir = std::env::temp_dir().join(format!("lf-warm-{}-{porta}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let base = format!("http://127.0.0.1:{porta}");
        let urls = vec![
            format!("{base}/0"), // imagem boa
            format!("{base}/1"), // HTML disfarçado de <img>
            format!("{base}/2"), // imagem acima do teto
        ];

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        warm(&client, &dir, &urls);
        assert_eq!(hits.load(Ordering::SeqCst), 3, "as 3 foram buscadas");

        let map = data_uris(&dir, &urls);
        // Só a imagem de verdade entrou no cache.
        assert_eq!(map.len(), 1);
        let uri = map.get(&urls[0]).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"));

        // Segunda passada: o que já está em disco NÃO é rebaixado. Esse é o
        // ponto do cache — sem isso cada abertura do artigo avisa o servidor.
        warm(&client, &dir, &urls);
        let depois = hits.load(Ordering::SeqCst);
        assert_eq!(depois, 5, "só as 2 que falharam foram tentadas de novo");

        // E o HTML final não guarda nenhuma URL http pra imagem cacheada.
        let html = format!(r#"<img src="{}">"#, urls[0]);
        let out = rewrite(&html, &map);
        assert!(!out.contains("http://"), "sobrou requisição remota: {out}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_ida_e_volta_no_disco() {
        let dir = std::env::temp_dir().join(format!("lf-img-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let url = "https://a.com/1.png";
        let mut buf = b"image/png\n".to_vec();
        buf.extend_from_slice(&[1, 2, 3, 4]);
        std::fs::write(dir.join(key(url)), buf).unwrap();

        let map = data_uris(&dir, &[url.to_string()]);
        assert_eq!(map.get(url).map(String::as_str), Some("data:image/png;base64,AQIDBA=="));

        assert!(dir_bytes(&dir) > 0);
        assert!(clear(&dir) > 0);
        assert!(data_uris(&dir, &[url.to_string()]).is_empty());
    }
}
