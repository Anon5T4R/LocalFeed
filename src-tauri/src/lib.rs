mod db;
mod fetch;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use db::{ArticleFilter, ArticleFull, ArticleRow, FeedRow};
use fetch::now_ms;

pub struct Db(Mutex<Option<Connection>>);

fn with_conn<T>(
    db: &State<'_, Db>,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard = db.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("banco não inicializado")?;
    f(conn)
}

// ---------- feeds ----------

#[tauri::command(async)]
fn list_feeds(db: State<'_, Db>) -> Result<Vec<FeedRow>, String> {
    with_conn(&db, db::list_feeds)
}

#[tauri::command(async)]
fn add_feed(db: State<'_, Db>, url: String) -> Result<FeedRow, String> {
    let client = fetch::client()?;
    let found = fetch::discover(&client, url.trim())?;
    let title = fetch::feed_title(&found.feed, &found.feed_url);
    let site = fetch::site_url(&found.feed);
    with_conn(&db, |conn| {
        conn.execute(
            "INSERT INTO feeds (url, title, site_url, added_ms, last_fetch_ms)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(url) DO UPDATE SET title = excluded.title",
            rusqlite::params![found.feed_url, title, site, now_ms()],
        )
        .map_err(|e| e.to_string())?;
        let id: i64 = conn
            .query_row("SELECT id FROM feeds WHERE url = ?1", [&found.feed_url], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        fetch::upsert_articles(conn, id, &found.feed)?;
        let unread: i64 = conn
            .query_row("SELECT COUNT(*) FROM articles WHERE feed_id = ?1 AND read = 0", [id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(FeedRow { id, url: found.feed_url.clone(), title, site_url: site, folder: None, unread, last_error: None })
    })
}

#[tauri::command(async)]
fn remove_feed(db: State<'_, Db>, feed_id: i64) -> Result<(), String> {
    with_conn(&db, |conn| {
        conn.execute("DELETE FROM feeds WHERE id = ?1", [feed_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

/// Move um feed pra uma pasta (folder vazio/None = sem pasta).
#[tauri::command(async)]
fn set_feed_folder(db: State<'_, Db>, feed_id: i64, folder: Option<String>) -> Result<(), String> {
    with_conn(&db, |conn| db::set_feed_folder(conn, feed_id, folder))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RefreshSummary {
    new_articles: u32,
    errors: Vec<String>,
}

/// Atualiza todos os feeds (rede FORA do lock do banco; upsert dentro).
#[tauri::command(async)]
fn refresh_all(app: AppHandle, db: State<'_, Db>) -> Result<RefreshSummary, String> {
    let feeds = with_conn(&db, db::list_feeds)?;
    let client = fetch::client()?;
    let mut summary = RefreshSummary { new_articles: 0, errors: vec![] };
    for feed in feeds {
        let _ = app.emit("refresh-progress", feed.title.clone());
        match fetch::discover(&client, &feed.url) {
            Ok(found) => {
                let added = with_conn(&db, |conn| {
                    let n = fetch::upsert_articles(conn, feed.id, &found.feed)?;
                    conn.execute(
                        "UPDATE feeds SET last_fetch_ms = ?1, last_error = NULL WHERE id = ?2",
                        rusqlite::params![now_ms(), feed.id],
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(n)
                })?;
                summary.new_articles += added;
            }
            Err(e) => {
                let _ = with_conn(&db, |conn| {
                    conn.execute(
                        "UPDATE feeds SET last_error = ?1 WHERE id = ?2",
                        rusqlite::params![e, feed.id],
                    )
                    .map_err(|er| er.to_string())?;
                    Ok(())
                });
                summary.errors.push(format!("{}: {e}", feed.title));
            }
        }
    }
    Ok(summary)
}

// ---------- artigos ----------

#[tauri::command(async)]
fn list_articles(
    db: State<'_, Db>,
    feed_id: Option<i64>,
    unread_only: bool,
    favorites_only: bool,
) -> Result<Vec<ArticleRow>, String> {
    with_conn(&db, |conn| {
        db::list_articles(conn, &ArticleFilter { feed_id, unread_only, favorites_only })
    })
}

/// Artigo completo; sem conteúdo cacheado, busca a página e extrai o texto
/// limpo (readability) — cai pro resumo do feed se falhar.
#[tauri::command(async)]
fn get_article(db: State<'_, Db>, article_id: i64) -> Result<ArticleFull, String> {
    let (url, cached): (Option<String>, Option<String>) = with_conn(&db, |conn| {
        conn.query_row(
            "SELECT url, content FROM articles WHERE id = ?1",
            [article_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())
    })?;

    if cached.is_none() {
        if let Some(u) = &url {
            if let Ok(client) = fetch::client() {
                if let Ok(content) = fetch::extract_readable(&client, u) {
                    let _ = with_conn(&db, |conn| {
                        conn.execute(
                            "UPDATE articles SET content = ?1 WHERE id = ?2",
                            rusqlite::params![content, article_id],
                        )
                        .map_err(|e| e.to_string())?;
                        Ok(())
                    });
                }
            }
        }
    }

    with_conn(&db, |conn| {
        conn.query_row(
            "SELECT a.id, a.feed_id, f.title, a.title, a.url, a.author, a.published_ms,
                    COALESCE(a.content, a.summary), a.read, a.favorite
             FROM articles a JOIN feeds f ON f.id = a.feed_id WHERE a.id = ?1",
            [article_id],
            |r| {
                Ok(ArticleFull {
                    id: r.get(0)?,
                    feed_id: r.get(1)?,
                    feed_title: r.get(2)?,
                    title: r.get(3)?,
                    url: r.get(4)?,
                    author: r.get(5)?,
                    published_ms: r.get(6)?,
                    content_html: r.get(7)?,
                    read: r.get::<_, i64>(8)? != 0,
                    favorite: r.get::<_, i64>(9)? != 0,
                })
            },
        )
        .map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn mark_read(db: State<'_, Db>, article_id: i64, read: bool) -> Result<(), String> {
    with_conn(&db, |conn| {
        conn.execute(
            "UPDATE articles SET read = ?1 WHERE id = ?2",
            rusqlite::params![read as i64, article_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command(async)]
fn mark_all_read(db: State<'_, Db>, feed_id: Option<i64>) -> Result<(), String> {
    with_conn(&db, |conn| {
        match feed_id {
            Some(id) => conn.execute("UPDATE articles SET read = 1 WHERE feed_id = ?1", [id]),
            None => conn.execute("UPDATE articles SET read = 1", []),
        }
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command(async)]
fn toggle_favorite(db: State<'_, Db>, article_id: i64) -> Result<bool, String> {
    with_conn(&db, |conn| {
        conn.execute(
            "UPDATE articles SET favorite = 1 - favorite WHERE id = ?1",
            [article_id],
        )
        .map_err(|e| e.to_string())?;
        conn.query_row("SELECT favorite FROM articles WHERE id = ?1", [article_id], |r| {
            r.get::<_, i64>(0).map(|v| v != 0)
        })
        .map_err(|e| e.to_string())
    })
}

// ---------- dados e armazenamento ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StorageInfo {
    /// Pasta de dados do app (onde mora o localfeed.db).
    dir: String,
    /// Tamanho do banco em bytes (db + WAL + SHM).
    db_bytes: u64,
    articles: i64,
    cached: i64,
    favorites: i64,
}

#[tauri::command(async)]
fn storage_info(app: AppHandle, db: State<'_, Db>) -> Result<StorageInfo, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let counts = with_conn(&db, db::storage_counts)?;
    let db_bytes = ["localfeed.db", "localfeed.db-wal", "localfeed.db-shm"]
        .iter()
        .filter_map(|name| std::fs::metadata(dir.join(name)).ok())
        .map(|m| m.len())
        .sum();
    Ok(StorageInfo {
        dir: dir.to_string_lossy().into_owned(),
        db_bytes,
        articles: counts.articles,
        cached: counts.cached,
        favorites: counts.favorites,
    })
}

/// Limpa só o conteúdo readability em cache (artigos/lidos/favoritos ficam).
#[tauri::command(async)]
fn clear_readability_cache(db: State<'_, Db>) -> Result<u64, String> {
    with_conn(&db, db::clear_readability_cache)
}

/// Apaga artigos não favoritos com mais de `days` dias (favoritos nunca).
#[tauri::command(async)]
fn clear_old_articles(db: State<'_, Db>, days: u32) -> Result<u64, String> {
    let cutoff = now_ms() - i64::from(days) * 86_400_000;
    with_conn(&db, |conn| db::clear_old_articles(conn, cutoff))
}

// ---------- OPML ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OpmlImport {
    added: u32,
    skipped: u32,
}

/// Import de OPML: extrai os xmlUrl (parser leve — OPML é gerado por máquina)
/// e insere com o título do arquivo; os artigos vêm no próximo "Atualizar".
#[tauri::command(async)]
fn import_opml(db: State<'_, Db>, path: String) -> Result<OpmlImport, String> {
    let xml = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let mut result = OpmlImport { added: 0, skipped: 0 };
    with_conn(&db, |conn| {
        let mut pos = 0usize;
        let lower = xml.to_lowercase();
        while let Some(idx) = lower[pos..].find("<outline") {
            let start = pos + idx;
            let end = lower[start..].find('>').map(|e| start + e).unwrap_or(lower.len());
            let tag = &xml[start..end.min(xml.len())];
            if let Some(url) = attr(tag, "xmlUrl") {
                let title = attr(tag, "title")
                    .or_else(|| attr(tag, "text"))
                    .unwrap_or_else(|| url.clone());
                let n = conn
                    .execute(
                        "INSERT OR IGNORE INTO feeds (url, title, added_ms) VALUES (?1, ?2, ?3)",
                        rusqlite::params![url, title, now_ms()],
                    )
                    .map_err(|e| e.to_string())?;
                if n > 0 {
                    result.added += 1;
                } else {
                    result.skipped += 1;
                }
            }
            pos = end.max(start + 8);
            if pos >= lower.len() {
                break;
            }
        }
        Ok(())
    })?;
    Ok(result)
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let at = lower.find(&format!("{}=", name.to_lowercase()))?;
    let rest = &tag[at + name.len() + 1..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(
        inner[..end]
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'"),
    )
}

#[tauri::command(async)]
fn export_opml(db: State<'_, Db>, path: String) -> Result<(), String> {
    let feeds = with_conn(&db, db::list_feeds)?;
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n  <head><title>LocalFeed</title></head>\n  <body>\n",
    );
    for f in feeds {
        xml.push_str(&format!(
            "    <outline type=\"rss\" text=\"{}\" title=\"{}\" xmlUrl=\"{}\"/>\n",
            esc(&f.title),
            esc(&f.title),
            esc(&f.url)
        ));
    }
    xml.push_str("  </body>\n</opml>\n");
    std::fs::write(&path, xml).map_err(|e| format!("{path}: {e}"))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// Arquivo `.opml` passado no launch (associação), se houver.
#[tauri::command(async)]
fn get_startup_file() -> Option<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .find(|a| a.to_lowercase().ends_with(".opml") && Path::new(a).is_file())
}

// ---------- bootstrap ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_focus();
            }
            if let Some(f) = args.into_iter().skip(1).find(|a| a.to_lowercase().ends_with(".opml")) {
                let _ = app.emit("open-opml", f);
            }
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Db(Mutex::new(None)))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("localfeed.db")).map_err(std::io::Error::other)?;
            *app.state::<Db>().0.lock().unwrap() = Some(conn);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_feeds,
            add_feed,
            remove_feed,
            set_feed_folder,
            refresh_all,
            list_articles,
            get_article,
            mark_read,
            mark_all_read,
            toggle_favorite,
            import_opml,
            export_opml,
            get_startup_file,
            storage_info,
            clear_readability_cache,
            clear_old_articles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attr_extrai_xmlurl() {
        let tag = r#"<outline type="rss" text="Blog &amp; Cia" xmlUrl="https://ex.com/feed"/>"#;
        assert_eq!(attr(tag, "xmlUrl"), Some("https://ex.com/feed".into()));
        assert_eq!(attr(tag, "text"), Some("Blog & Cia".into()));
        assert_eq!(attr(tag, "nada"), None);
    }

    #[test]
    fn esc_escapa_xml() {
        assert_eq!(esc(r#"a & "b" <c>"#), "a &amp; &quot;b&quot; &lt;c>");
    }
}
