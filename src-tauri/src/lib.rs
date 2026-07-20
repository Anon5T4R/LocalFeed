mod db;
mod fetch;
mod search;

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use db::{ArticleFilter, ArticleFull, ArticleRow, FeedRow};
use fetch::now_ms;
use search::{FtIndex, SearchHit, SearchOpts};

pub struct Db(Mutex<Option<Connection>>);

/// Índice full-text + progresso do backfill inicial.
///
/// **REGRA DE CADEADO: `Db` sempre antes de `Ft`, nunca o contrário.**
/// O backfill (que precisa dos dois) roda em outro thread ao mesmo tempo que
/// o usuário busca; se a busca pegasse `Ft` e depois `Db` enquanto o backfill
/// segura `Db` esperando `Ft`, os dois travavam de vez. Quem só precisa de um
/// deles pega só aquele — o `index_ids` inclusive solta o `Db` antes de pegar
/// o `Ft`, pra não segurar o banco durante a escrita no índice.
pub struct Ft {
    index: Mutex<Option<FtIndex>>,
    /// Backfill rodando (primeira execução de quem já tinha artigos).
    building: AtomicBool,
    done: AtomicU32,
    total: AtomicU32,
}

impl Ft {
    fn new() -> Ft {
        Ft {
            index: Mutex::new(None),
            building: AtomicBool::new(false),
            done: AtomicU32::new(0),
            total: AtomicU32::new(0),
        }
    }
}

fn with_conn<T>(
    db: &State<'_, Db>,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard = db.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("banco não inicializado")?;
    f(conn)
}

/// Indexa os artigos de `ids` (lê do banco, solta o cadeado, escreve no
/// índice). Erro aqui não é fatal: o índice é derivado e o `reconcile` do
/// próximo boot recupera o que ficou pra trás.
fn index_ids(app: &AppHandle, ids: &[i64]) {
    index_ids_inner(app, ids, true)
}

/// `commit = false` deixa os documentos enfileirados no writer: é o backfill,
/// que fecha o commit a cada N lotes (commit dispara merge, e merge em
/// excesso é o que faz o Windows brigar por arquivo mapeado).
fn index_ids_inner(app: &AppHandle, ids: &[i64], commit: bool) {
    if ids.is_empty() {
        return;
    }
    let docs = {
        let db = app.state::<Db>();
        let guard = db.0.lock().unwrap();
        match guard.as_ref() {
            Some(conn) => search::fetch_docs(conn, ids).unwrap_or_default(),
            None => return,
        }
    };
    let ft = app.state::<Ft>();
    let mut guard = ft.index.lock().unwrap();
    if let Some(i) = guard.as_mut() {
        let _ = if commit {
            i.index_docs(&docs)
        } else {
            i.add_docs(&docs)
        };
    }
}

fn commit_index(app: &AppHandle) {
    let ft = app.state::<Ft>();
    let mut guard = ft.index.lock().unwrap();
    if let Some(i) = guard.as_mut() {
        let _ = i.commit();
    }
}

fn unindex_ids(app: &AppHandle, ids: &[i64]) {
    if ids.is_empty() {
        return;
    }
    let ft = app.state::<Ft>();
    let mut guard = ft.index.lock().unwrap();
    if let Some(i) = guard.as_mut() {
        let _ = i.delete_ids(ids);
    }
}

// ---------- feeds ----------

#[tauri::command(async)]
fn list_feeds(db: State<'_, Db>) -> Result<Vec<FeedRow>, String> {
    with_conn(&db, db::list_feeds)
}

#[tauri::command(async)]
fn add_feed(app: AppHandle, db: State<'_, Db>, url: String) -> Result<FeedRow, String> {
    let client = fetch::client()?;
    let found = fetch::discover(&client, url.trim())?;
    let title = fetch::feed_title(&found.feed, &found.feed_url);
    let site = fetch::site_url(&found.feed);
    let (row, novos) = with_conn(&db, |conn| {
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
        let novos = fetch::upsert_articles(conn, id, &found.feed)?;
        let unread: i64 = conn
            .query_row("SELECT COUNT(*) FROM articles WHERE feed_id = ?1 AND read = 0", [id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok((
            FeedRow { id, url: found.feed_url.clone(), title, site_url: site, folder: None, unread, last_error: None },
            novos,
        ))
    })?;
    index_ids(&app, &novos);
    Ok(row)
}

#[tauri::command(async)]
fn remove_feed(app: AppHandle, db: State<'_, Db>, feed_id: i64) -> Result<(), String> {
    // Colhe os ids ANTES: o ON DELETE CASCADE leva os artigos embora sem dizer
    // quais eram, e o índice ficaria com órfãos até o próximo boot.
    let ids = with_conn(&db, |conn| {
        let ids = db::article_ids_of_feed(conn, feed_id)?;
        conn.execute("DELETE FROM feeds WHERE id = ?1", [feed_id])
            .map_err(|e| e.to_string())?;
        Ok(ids)
    })?;
    unindex_ids(&app, &ids);
    Ok(())
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
                let novos = with_conn(&db, |conn| {
                    let ids = fetch::upsert_articles(conn, feed.id, &found.feed)?;
                    conn.execute(
                        "UPDATE feeds SET last_fetch_ms = ?1, last_error = NULL WHERE id = ?2",
                        rusqlite::params![now_ms(), feed.id],
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(ids)
                })?;
                summary.new_articles += novos.len() as u32;
                // Índice incremental: só o que chegou agora entra.
                index_ids(&app, &novos);
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
fn get_article(app: AppHandle, db: State<'_, Db>, article_id: i64) -> Result<ArticleFull, String> {
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
                    let ok = with_conn(&db, |conn| {
                        conn.execute(
                            "UPDATE articles SET content = ?1 WHERE id = ?2",
                            rusqlite::params![content, article_id],
                        )
                        .map_err(|e| e.to_string())?;
                        Ok(())
                    });
                    // O artigo estava indexado só pelo resumo do feed; agora
                    // que o texto completo chegou, o índice acompanha (o
                    // index_docs substitui o documento, não duplica).
                    if ok.is_ok() {
                        index_ids(&app, &[article_id]);
                    }
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
    /// Tamanho do índice de busca em bytes (derivado — dá pra apagar).
    index_bytes: u64,
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
        index_bytes: search::index_bytes(&search::index_dir(&dir)),
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
fn clear_old_articles(app: AppHandle, db: State<'_, Db>, days: u32) -> Result<u64, String> {
    let cutoff = now_ms() - i64::from(days) * 86_400_000;
    let ids = with_conn(&db, |conn| db::clear_old_articles(conn, cutoff))?;
    unindex_ids(&app, &ids);
    Ok(ids.len() as u64)
}

// ---------- busca full-text ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SearchStatus {
    /// Backfill inicial em andamento (quem já tinha artigos ganha o índice
    /// na primeira execução).
    building: bool,
    done: u32,
    total: u32,
    /// Documentos no índice (0 e `building=false` = índice vazio mesmo).
    docs: u64,
}

#[tauri::command(async)]
fn search_status(ft: State<'_, Ft>) -> Result<SearchStatus, String> {
    let docs = match ft.index.try_lock() {
        // O backfill segura o cadeado em lotes curtos; se estiver ocupado,
        // não vale a pena bloquear a UI só pra contar documento.
        Ok(g) => g.as_ref().map(|i| i.doc_count()).unwrap_or(0),
        Err(_) => 0,
    };
    Ok(SearchStatus {
        building: ft.building.load(Ordering::Relaxed),
        done: ft.done.load(Ordering::Relaxed),
        total: ft.total.load(Ordering::Relaxed),
        docs,
    })
}

/// Busca full-text. Os filtros são aplicados no SQLite (ver search.rs).
#[tauri::command(async)]
fn search_articles(
    db: State<'_, Db>,
    ft: State<'_, Ft>,
    query: String,
    feed_id: Option<i64>,
    unread_only: bool,
    favorites_only: bool,
    since_ms: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let opts = SearchOpts {
        query,
        feed_id,
        unread_only,
        favorites_only,
        since_ms,
        limit: limit.unwrap_or(50),
    };
    // Db antes de Ft (ver a regra em `struct Ft`) — inverter aqui trava o app
    // junto com o backfill.
    let db_guard = db.0.lock().unwrap();
    let conn = db_guard.as_ref().ok_or("banco não inicializado")?;
    let ft_guard = ft.index.lock().unwrap();
    let index = ft_guard.as_ref().ok_or("índice de busca indisponível")?;
    search::search(conn, index, &opts)
}

/// Abre o índice e o põe em dia com o banco, em segundo plano.
///
/// É o caminho da ATUALIZAÇÃO: quem já tem artigos chega aqui sem índice
/// nenhum e ganha um, em lotes, com progresso — o boot não espera por isso.
/// Também é o conserto automático de índice truncado/apagado/órfão.
fn spawn_reconcile(app: AppHandle) {
    std::thread::spawn(move || {
        let Ok(dir) = app.path().app_data_dir() else {
            return;
        };
        let index = match search::open_or_create(&search::index_dir(&dir)) {
            Ok(i) => i,
            Err(e) => {
                let _ = app.emit("search-index", format!("erro: {e}"));
                return;
            }
        };
        {
            let ft = app.state::<Ft>();
            *ft.index.lock().unwrap() = Some(index);
        }

        let plan = {
            let db = app.state::<Db>();
            let ft = app.state::<Ft>();
            let db_guard = db.0.lock().unwrap();
            let Some(conn) = db_guard.as_ref() else { return };
            let ft_guard = ft.index.lock().unwrap();
            let Some(i) = ft_guard.as_ref() else { return };
            search::reconcile_plan(conn, i)
        };
        let Ok((missing, orphans)) = plan else { return };

        unindex_ids(&app, &orphans);
        if missing.is_empty() {
            let _ = app.emit("search-index", SearchStatus { building: false, done: 0, total: 0, docs: 0 });
            return;
        }

        let ft = app.state::<Ft>();
        ft.building.store(true, Ordering::Relaxed);
        ft.total.store(missing.len() as u32, Ordering::Relaxed);
        ft.done.store(0, Ordering::Relaxed);
        // Lotes: cada um pega e solta os dois cadeados, então buscar e ler
        // artigo continuam respondendo enquanto o índice é construído.
        for (i, chunk) in missing.chunks(200).enumerate() {
            index_ids_inner(&app, chunk, false);
            if (i + 1) % 10 == 0 {
                commit_index(&app);
            }
            let done = ft.done.fetch_add(chunk.len() as u32, Ordering::Relaxed) + chunk.len() as u32;
            let _ = app.emit(
                "search-index",
                SearchStatus { building: true, done, total: missing.len() as u32, docs: 0 },
            );
        }
        commit_index(&app);
        ft.building.store(false, Ordering::Relaxed);
        let _ = app.emit(
            "search-index",
            SearchStatus {
                building: false,
                done: missing.len() as u32,
                total: missing.len() as u32,
                docs: 0,
            },
        );
    });
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
        .manage(Ft::new())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            // ORDEM IMPORTA: o banco (com as migrações) precisa estar pronto
            // antes de qualquer coisa ler `articles` — o índice é derivado
            // dele. Por isso o reconcile vai depois, e em outro thread.
            let conn = db::open(&dir.join("localfeed.db")).map_err(std::io::Error::other)?;
            *app.state::<Db>().0.lock().unwrap() = Some(conn);
            spawn_reconcile(app.handle().clone());
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
            search_articles,
            search_status,
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
