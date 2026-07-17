//! SQLite único em app_data (agenda não é documento — feeds também não).
//! Schema versionado via `user_version`, mesmo padrão do LocalData/Agenda.

use rusqlite::Connection;
use serde::Serialize;

pub fn open(path: &std::path::Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )
    .map_err(|e| e.to_string())?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), String> {
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if v < 1 {
        conn.execute_batch(
            "CREATE TABLE feeds (
               id INTEGER PRIMARY KEY,
               url TEXT NOT NULL UNIQUE,
               title TEXT NOT NULL,
               site_url TEXT,
               added_ms INTEGER NOT NULL,
               last_fetch_ms INTEGER,
               last_error TEXT
             );
             CREATE TABLE articles (
               id INTEGER PRIMARY KEY,
               feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
               guid TEXT NOT NULL,
               title TEXT NOT NULL,
               url TEXT,
               author TEXT,
               published_ms INTEGER,
               excerpt TEXT NOT NULL DEFAULT '',
               summary TEXT,
               content TEXT,
               read INTEGER NOT NULL DEFAULT 0,
               favorite INTEGER NOT NULL DEFAULT 0,
               fetched_ms INTEGER NOT NULL,
               UNIQUE(feed_id, guid)
             );
             CREATE INDEX idx_articles_feed ON articles(feed_id, published_ms DESC);
             CREATE INDEX idx_articles_read ON articles(read);
             PRAGMA user_version = 1;",
        )
        .map_err(|e| e.to_string())?;
    }
    if v < 2 {
        // Pastas: cada feed pode pertencer a uma (NULL = sem pasta).
        conn.execute_batch(
            "ALTER TABLE feeds ADD COLUMN folder TEXT;
             PRAGMA user_version = 2;",
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FeedRow {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub site_url: Option<String>,
    pub folder: Option<String>,
    pub unread: i64,
    pub last_error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleRow {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: String,
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    pub published_ms: Option<i64>,
    pub excerpt: String,
    pub read: bool,
    pub favorite: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleFull {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: String,
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    pub published_ms: Option<i64>,
    pub content_html: Option<String>,
    pub read: bool,
    pub favorite: bool,
}

pub fn list_feeds(conn: &Connection) -> Result<Vec<FeedRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.url, f.title, f.site_url, f.folder, f.last_error,
                    (SELECT COUNT(*) FROM articles a WHERE a.feed_id = f.id AND a.read = 0)
             FROM feeds f ORDER BY f.title COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(FeedRow {
                id: r.get(0)?,
                url: r.get(1)?,
                title: r.get(2)?,
                site_url: r.get(3)?,
                folder: r.get(4)?,
                last_error: r.get(5)?,
                unread: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Define (ou limpa, se vazio) a pasta de um feed.
pub fn set_feed_folder(conn: &Connection, feed_id: i64, folder: Option<String>) -> Result<(), String> {
    let f = folder.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    conn.execute(
        "UPDATE feeds SET folder = ?2 WHERE id = ?1",
        rusqlite::params![feed_id, f],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub struct ArticleFilter {
    pub feed_id: Option<i64>,
    pub unread_only: bool,
    pub favorites_only: bool,
}

pub fn list_articles(conn: &Connection, f: &ArticleFilter) -> Result<Vec<ArticleRow>, String> {
    let mut sql = String::from(
        "SELECT a.id, a.feed_id, f.title, a.title, a.url, a.author, a.published_ms,
                a.excerpt, a.read, a.favorite
         FROM articles a JOIN feeds f ON f.id = a.feed_id WHERE 1=1",
    );
    if f.feed_id.is_some() {
        sql.push_str(" AND a.feed_id = ?1");
    }
    if f.unread_only {
        sql.push_str(" AND a.read = 0");
    }
    if f.favorites_only {
        sql.push_str(" AND a.favorite = 1");
    }
    sql.push_str(" ORDER BY a.published_ms DESC NULLS LAST, a.id DESC LIMIT 500");

    let map = |r: &rusqlite::Row| {
        Ok(ArticleRow {
            id: r.get(0)?,
            feed_id: r.get(1)?,
            feed_title: r.get(2)?,
            title: r.get(3)?,
            url: r.get(4)?,
            author: r.get(5)?,
            published_ms: r.get(6)?,
            excerpt: r.get(7)?,
            read: r.get::<_, i64>(8)? != 0,
            favorite: r.get::<_, i64>(9)? != 0,
        })
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = if let Some(id) = f.feed_id {
        stmt.query_map([id], map).map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
    } else {
        stmt.query_map([], map).map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
    };
    rows.map_err(|e| e.to_string())
}
