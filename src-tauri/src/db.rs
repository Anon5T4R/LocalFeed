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

/// `pub(crate)` porque o módulo `search` monta bancos de teste com o mesmo
/// schema — a migração é a única fonte de verdade dele.
pub(crate) fn migrate(conn: &Connection) -> Result<(), String> {
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
    if v < 3 {
        // "Ler depois": intenção explícita do usuário, separada de favorito.
        // Favorito é "quero guardar"; ler-depois é "quero voltar aqui". Sai só
        // por ação do usuário — desmarcar sozinho ao abrir seria adivinhar
        // (abrir e fechar sem ler é comum). A limpeza automática respeita as
        // duas (ver `clear_old_articles`): apagar um artigo que o usuário
        // marcou pra ler é perda silenciosa do pior tipo.
        conn.execute_batch(
            "ALTER TABLE articles ADD COLUMN later INTEGER NOT NULL DEFAULT 0;
             CREATE INDEX idx_articles_later ON articles(later) WHERE later = 1;
             PRAGMA user_version = 3;",
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
    pub later: bool,
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
    pub later: bool,
}

/// Contagens pro painel "Dados e armazenamento".
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageCounts {
    pub articles: i64,
    /// Artigos com conteúdo readability em cache (coluna `content`).
    pub cached: i64,
    pub favorites: i64,
    /// Marcados pra ler depois — como os favoritos, sobrevivem à limpeza.
    pub later: i64,
}

pub fn storage_counts(conn: &Connection) -> Result<StorageCounts, String> {
    conn.query_row(
        "SELECT COUNT(*),
                COUNT(content),
                SUM(favorite),
                SUM(later)
         FROM articles",
        [],
        |r| {
            Ok(StorageCounts {
                articles: r.get(0)?,
                cached: r.get(1)?,
                favorites: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                later: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// Limpa SÓ o conteúdo readability em cache. Os artigos e o estado de
/// lido/favorito FICAM — o texto completo volta a ser baixado na abertura.
/// Retorna quantos artigos tiveram o cache limpo.
pub fn clear_readability_cache(conn: &Connection) -> Result<u64, String> {
    let n = conn
        .execute("UPDATE articles SET content = NULL WHERE content IS NOT NULL", [])
        .map_err(|e| e.to_string())?;
    conn.execute_batch("VACUUM").map_err(|e| e.to_string())?;
    Ok(n as u64)
}

/// Ids dos artigos de um feed (colhidos ANTES do DELETE, pra poder tirá-los
/// do índice de busca — o CASCADE do SQLite não avisa quem foi embora).
pub fn article_ids_of_feed(conn: &Connection, feed_id: i64) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare("SELECT id FROM articles WHERE feed_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([feed_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Apaga artigos mais velhos que `cutoff_ms` que NÃO são favoritos **nem estão
/// marcados pra ler depois** (feeds ficam intactos). Usa a data de publicação,
/// caindo pra data de download quando o feed não informa.
/// Retorna os ids apagados (o chamador os tira do índice de busca).
pub fn clear_old_articles(conn: &Connection, cutoff_ms: i64) -> Result<Vec<i64>, String> {
    let cond = "favorite = 0 AND later = 0 AND COALESCE(published_ms, fetched_ms) < ?1";
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare(&format!("SELECT id FROM articles WHERE {cond}"))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([cutoff_ms], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
    };
    conn.execute(&format!("DELETE FROM articles WHERE {cond}"), [cutoff_ms])
        .map_err(|e| e.to_string())?;
    conn.execute_batch("VACUUM").map_err(|e| e.to_string())?;
    Ok(ids)
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
    pub later_only: bool,
}

pub fn list_articles(conn: &Connection, f: &ArticleFilter) -> Result<Vec<ArticleRow>, String> {
    let mut sql = String::from(
        "SELECT a.id, a.feed_id, f.title, a.title, a.url, a.author, a.published_ms,
                a.excerpt, a.read, a.favorite, a.later
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
    if f.later_only {
        sql.push_str(" AND a.later = 1");
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
            later: r.get::<_, i64>(10)? != 0,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO feeds (id, url, title, added_ms) VALUES (1, 'https://ex.com/feed', 'Ex', 0)",
            [],
        )
        .unwrap();
        conn
    }

    fn insert_article(
        conn: &Connection,
        guid: &str,
        published_ms: Option<i64>,
        fetched_ms: i64,
        favorite: bool,
        content: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO articles (feed_id, guid, title, published_ms, fetched_ms, favorite, read, content)
             VALUES (1, ?1, ?1, ?2, ?3, ?4, 1, ?5)",
            rusqlite::params![guid, published_ms, fetched_ms, favorite as i64, content],
        )
        .unwrap();
    }

    /// Banco na v2 EXATAMENTE como o app antigo o deixou — schema escrito à
    /// mão, sem passar pela `migrate()` de hoje. É o único jeito de exercitar
    /// o caminho da ATUALIZAÇÃO: um banco criado do zero nunca roda o ALTER.
    fn conn_v2_com_dados() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE feeds (
               id INTEGER PRIMARY KEY, url TEXT NOT NULL UNIQUE, title TEXT NOT NULL,
               site_url TEXT, added_ms INTEGER NOT NULL, last_fetch_ms INTEGER,
               last_error TEXT, folder TEXT
             );
             CREATE TABLE articles (
               id INTEGER PRIMARY KEY,
               feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
               guid TEXT NOT NULL, title TEXT NOT NULL, url TEXT, author TEXT,
               published_ms INTEGER, excerpt TEXT NOT NULL DEFAULT '',
               summary TEXT, content TEXT,
               read INTEGER NOT NULL DEFAULT 0, favorite INTEGER NOT NULL DEFAULT 0,
               fetched_ms INTEGER NOT NULL, UNIQUE(feed_id, guid)
             );
             INSERT INTO feeds (id, url, title, added_ms, folder)
               VALUES (1, 'https://ex.com/feed', 'Ex', 0, 'Notícias');
             INSERT INTO articles (id, feed_id, guid, title, fetched_ms, read, favorite)
               VALUES (7, 1, 'g1', 'Artigo antigo', 100, 1, 1);
             PRAGMA user_version = 2;",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migracao_2_para_3_preserva_dados_e_zera_o_later() {
        let conn = conn_v2_com_dados();
        migrate(&conn).unwrap();

        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);

        // A linha que já existia continua lá, com o estado dela intacto...
        let (titulo, read, fav, later): (String, i64, i64, i64) = conn
            .query_row(
                "SELECT title, read, favorite, later FROM articles WHERE id = 7",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(titulo, "Artigo antigo");
        assert_eq!((read, fav), (1, 1));
        // ...e a coluna nova nasce em 0, não NULL: `later = 0` no WHERE da
        // limpeza descartaria a linha inteira se fosse NULL.
        assert_eq!(later, 0);

        // A pasta da v2 sobreviveu (o ALTER da v3 não recriou a tabela).
        let pasta: Option<String> = conn
            .query_row("SELECT folder FROM feeds WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(pasta.as_deref(), Some("Notícias"));
    }

    #[test]
    fn migracao_e_idempotente() {
        let conn = conn_v2_com_dados();
        migrate(&conn).unwrap();
        // Rodar de novo (app reaberto) não pode explodir no ALTER duplicado.
        migrate(&conn).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM articles", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn clear_old_articles_preserva_ler_depois() {
        let conn = test_conn();
        insert_article(&conn, "velho", Some(100), 100, false, None);
        insert_article(&conn, "velho-ler-depois", Some(100), 100, false, None);
        conn.execute(
            "UPDATE articles SET later = 1 WHERE guid = 'velho-ler-depois'",
            [],
        )
        .unwrap();

        let apagados = clear_old_articles(&conn, 1_000).unwrap();
        assert_eq!(apagados.len(), 1);

        let restantes: Vec<String> = {
            let mut stmt = conn.prepare("SELECT guid FROM articles").unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(restantes, vec!["velho-ler-depois".to_string()]);
    }

    #[test]
    fn clear_old_articles_preserva_favoritos_e_recentes() {
        let conn = test_conn();
        insert_article(&conn, "velho", Some(100), 100, false, None);
        insert_article(&conn, "velho-fav", Some(100), 100, true, None);
        insert_article(&conn, "recente", Some(9_000), 9_000, false, None);
        // Sem published_ms: cai pro fetched_ms.
        insert_article(&conn, "velho-sem-data", None, 100, false, None);

        let apagados = clear_old_articles(&conn, 1_000).unwrap();
        assert_eq!(apagados.len(), 2); // "velho" e "velho-sem-data"

        let restantes: Vec<String> = {
            let mut stmt = conn.prepare("SELECT guid FROM articles ORDER BY guid").unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(restantes, vec!["recente".to_string(), "velho-fav".to_string()]);

        // O feed fica intacto.
        let feeds: i64 = conn.query_row("SELECT COUNT(*) FROM feeds", [], |r| r.get(0)).unwrap();
        assert_eq!(feeds, 1);
    }

    #[test]
    fn clear_readability_cache_mantem_lido_e_favorito() {
        let conn = test_conn();
        insert_article(&conn, "a", Some(100), 100, true, Some("<p>cache</p>"));
        insert_article(&conn, "b", Some(200), 200, false, None);

        assert_eq!(storage_counts(&conn).unwrap().cached, 1);
        let n = clear_readability_cache(&conn).unwrap();
        assert_eq!(n, 1);

        let (count, cached, fav, read): (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COUNT(content), SUM(favorite), SUM(read) FROM articles",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!((count, cached, fav, read), (2, 0, 1, 2));
    }

    #[test]
    fn storage_counts_num_banco_vazio() {
        let conn = test_conn();
        let c = storage_counts(&conn).unwrap();
        assert_eq!((c.articles, c.cached, c.favorites), (0, 0, 0));
    }
}
