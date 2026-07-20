//! Rede do LocalFeed — a única exceção "não-offline" da suíte: buscar os
//! feeds que o usuário assinou (sem telemetria, sem tracker). Descoberta de
//! feed em página HTML, refresh com upsert por guid e extração de artigo
//! limpo (readability) cacheada no SQLite.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("LocalFeed/0.1 (leitor RSS offline; +https://github.com/Anon5T4R/LocalFeed)")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

fn get_bytes(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().map_err(|e| friendly(url, e))?;
    if !resp.status().is_success() {
        return Err(format!("{url}: HTTP {}", resp.status().as_u16()));
    }
    resp.bytes().map(|b| b.to_vec()).map_err(|e| friendly(url, e))
}

fn friendly(url: &str, e: reqwest::Error) -> String {
    if e.is_timeout() {
        format!("{url}: tempo esgotado")
    } else if e.is_connect() {
        format!("{url}: sem conexão")
    } else {
        format!("{url}: {e}")
    }
}

/// Procura `<link rel="alternate" type="application/rss+xml" href=…>` num HTML.
fn html_feed_links(html: &str, base: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0usize;
    while let Some(idx) = lower[pos..].find("<link") {
        let start = pos + idx;
        let end = lower[start..].find('>').map(|e| start + e).unwrap_or(lower.len());
        let tag = &html[start..end.min(html.len())];
        let tag_lower = &lower[start..end.min(lower.len())];
        if tag_lower.contains("alternate")
            && (tag_lower.contains("rss+xml") || tag_lower.contains("atom+xml") || tag_lower.contains("feed+json"))
        {
            if let Some(href) = attr_value(tag, "href") {
                if let Ok(base_url) = url::Url::parse(base) {
                    if let Ok(abs) = base_url.join(&href) {
                        out.push(abs.to_string());
                    }
                }
            }
        }
        pos = end.min(lower.len().saturating_sub(1)).max(start + 5);
        if pos >= lower.len() {
            break;
        }
    }
    out
}

/// Valor de um atributo num fragmento de tag (aspas simples ou duplas).
fn attr_value(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let at = lower.find(&format!("{name}="))?;
    let rest = &tag[at + name.len() + 1..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

pub struct Discovered {
    pub feed_url: String,
    pub feed: feed_rs::model::Feed,
}

/// URL do usuário → feed de verdade: tenta como feed; se vier HTML, procura
/// os `<link rel=alternate>` e uns caminhos comuns (/feed, /rss…).
pub fn discover(client: &reqwest::blocking::Client, input: &str) -> Result<Discovered, String> {
    let url = if input.starts_with("http://") || input.starts_with("https://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let bytes = get_bytes(client, &url)?;
    if let Ok(feed) = feed_rs::parser::parse(&bytes[..]) {
        return Ok(Discovered { feed_url: url, feed });
    }
    // é HTML: procura os links declarados…
    let html = String::from_utf8_lossy(&bytes);
    let mut candidates = html_feed_links(&html, &url);
    // …e uns palpites comuns.
    for guess in ["feed", "rss", "feed.xml", "atom.xml", "index.xml", "rss.xml"] {
        if let Ok(base) = url::Url::parse(&url) {
            if let Ok(abs) = base.join(guess) {
                candidates.push(abs.to_string());
            }
        }
    }
    for cand in candidates {
        if let Ok(bytes) = get_bytes(client, &cand) {
            if let Ok(feed) = feed_rs::parser::parse(&bytes[..]) {
                return Ok(Discovered { feed_url: cand, feed });
            }
        }
    }
    Err("nenhum feed encontrado nesse endereço".into())
}

/// Texto puro de um HTML (excerpt) — remove tags e comprime espaços.
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len().min(4096));
    let mut in_tag = false;
    let mut last_space = true;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => {
                let ch = if c.is_whitespace() { ' ' } else { c };
                if ch == ' ' && last_space {
                    continue;
                }
                last_space = ch == ' ';
                out.push(ch);
            }
            _ => {}
        }
        if out.len() > 500 {
            break;
        }
    }
    // entidades mais comuns
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

/// Insere os artigos novos do feed (guid único por feed). Retorna os **ids**
/// dos inseridos — é o que alimenta o índice de busca incremental (só o que
/// chegou entra no índice; nada de reindexar tudo a cada atualização).
pub fn upsert_articles(
    conn: &Connection,
    feed_id: i64,
    feed: &feed_rs::model::Feed,
) -> Result<Vec<i64>, String> {
    let mut added: Vec<i64> = Vec::new();
    for entry in &feed.entries {
        let link = entry.links.first().map(|l| l.href.clone());
        let guid = if !entry.id.is_empty() {
            entry.id.clone()
        } else {
            link.clone().unwrap_or_else(|| {
                entry.title.as_ref().map(|t| t.content.clone()).unwrap_or_default()
            })
        };
        if guid.is_empty() {
            continue;
        }
        let title = entry
            .title
            .as_ref()
            .map(|t| strip_html(&t.content))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "(sem título)".into());
        let author = entry.authors.first().map(|a| a.name.clone());
        let published = entry
            .published
            .or(entry.updated)
            .map(|d| d.timestamp_millis());
        let summary = entry
            .summary
            .as_ref()
            .map(|s| s.content.clone())
            .or_else(|| {
                entry.content.as_ref().and_then(|c| c.body.clone())
            });
        let excerpt = summary.as_deref().map(strip_html).unwrap_or_default();
        let n = conn
            .execute(
                "INSERT OR IGNORE INTO articles
                 (feed_id, guid, title, url, author, published_ms, excerpt, summary, fetched_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![feed_id, guid, title, link, author, published, excerpt, summary, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        if n > 0 {
            added.push(conn.last_insert_rowid());
        }
    }
    Ok(added)
}

/// Título "limpo" do feed (fallback pro domínio).
pub fn feed_title(feed: &feed_rs::model::Feed, url: &str) -> String {
    feed.title
        .as_ref()
        .map(|t| strip_html(&t.content))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| url.to_string())
        })
}

pub fn site_url(feed: &feed_rs::model::Feed) -> Option<String> {
    feed.links.first().map(|l| l.href.clone())
}

/// Baixa a página do artigo e extrai o texto limpo (modo leitura).
pub fn extract_readable(client: &reqwest::blocking::Client, article_url: &str) -> Result<String, String> {
    let bytes = get_bytes(client, article_url)?;
    let parsed = url::Url::parse(article_url).map_err(|e| e.to_string())?;
    let mut cursor = std::io::Cursor::new(bytes);
    let product = readability::extractor::extract(&mut cursor, &parsed).map_err(|e| e.to_string())?;
    Ok(product.content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_remove_tags_e_entidades() {
        assert_eq!(strip_html("<p>Olá <b>mundo</b> &amp; cia</p>"), "Olá mundo & cia");
        assert_eq!(strip_html("um\n  dois\t tres"), "um dois tres");
    }

    #[test]
    fn html_feed_links_acha_alternate() {
        let html = r#"<html><head>
          <link rel="alternate" type="application/rss+xml" href="/feed.xml">
          <link rel="stylesheet" href="/style.css">
          <link rel="alternate" type="application/atom+xml" href="https://ex.com/atom">
        </head></html>"#;
        let links = html_feed_links(html, "https://ex.com/blog/");
        assert!(links.contains(&"https://ex.com/feed.xml".to_string()));
        assert!(links.contains(&"https://ex.com/atom".to_string()));
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn attr_value_aspas_simples_e_duplas() {
        assert_eq!(attr_value(r#"<link href="/a">"#, "href"), Some("/a".into()));
        assert_eq!(attr_value("<link href='/b'>", "href"), Some("/b".into()));
        assert_eq!(attr_value("<link>", "href"), None);
    }
}
