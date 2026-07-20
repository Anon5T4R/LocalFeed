//! Busca full-text dos artigos já baixados (tantivy).
//!
//! Porte do `search.rs` do LocalZIM — mesmo tokenizador (minúsculas + sem
//! acentos, pra "sao paulo" achar "São Paulo") e mesmo truque de **não
//! armazenar o corpo no índice**: o trecho destacado é re-extraído da fonte
//! na hora da busca. Lá a fonte é o .zim, aqui é o SQLite.
//!
//! Duas diferenças de fundo em relação ao LocalZIM:
//!
//! 1. **Incremental.** Lá o índice nasce de uma varredura única de um arquivo
//!    imutável; aqui chegam artigos o tempo todo, então o writer fica vivo e
//!    cada artigo novo entra sozinho (`index_articles`).
//! 2. **Nada de estado mutável no índice.** Lido/favorito mudam a toda hora e
//!    reindexar por causa disso seria absurdo — o tantivy devolve só os ids
//!    ordenados por relevância e o SQLite aplica os filtros. Consequência: o
//!    índice é 100% derivado (requisito: se sumir, reconstrói sem perder nada
//!    do usuário) e o `reconcile` sabe refazê-lo do zero.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, FAST, INDEXED,
};
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::{AsciiFoldingFilter, LowerCaser, SimpleTokenizer, TextAnalyzer};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term};

use crate::db::ArticleRow;

const TOKENIZER: &str = "folding";
/// 30 MB num único thread: o writer fica vivo a app inteira e cada commit
/// costuma ter poucas dezenas de artigos — heap grande aqui só desperdiça RAM.
const WRITER_HEAP: usize = 30 * 1024 * 1024;
const META_FILE: &str = "localfeed-index.json";
/// Muda quando o schema muda: o índice velho é apagado e reconstruído.
const SCHEMA_VERSION: u32 = 1;

/// Pasta do índice dentro do app_data (irmã do localfeed.db).
pub fn index_dir(app_data: &Path) -> std::path::PathBuf {
    app_data.join("search-index")
}

fn folding_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .build()
}

fn build_schema() -> Schema {
    let mut sb = Schema::builder();
    let text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(TOKENIZER)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    // INDEXED pra apagar por termo, FAST pra listar tudo que está indexado
    // (o `reconcile` varre a coluna inteira sem descomprimir documento).
    sb.add_u64_field("aid", INDEXED | FAST);
    sb.add_text_field("title", text.clone());
    sb.add_text_field("body", text.clone());
    sb.add_text_field("author", text.clone());
    sb.add_text_field("feed", text);
    sb.build()
}

pub struct FtIndex {
    index: Index,
    writer: IndexWriter,
    /// UM reader pra vida toda (recomendação do tantivy). Além de evitar o
    /// custo de reabrir, reduz o vaivém de mmap — que no Windows é o que faz
    /// o merge em segundo plano tropeçar (ver `commit`).
    reader: IndexReader,
    aid_f: Field,
    title_f: Field,
    body_f: Field,
    author_f: Field,
    feed_f: Field,
}

/// Um artigo pronto pra virar documento (texto já extraído do HTML).
pub struct IndexDoc {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub feed: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub article: ArticleRow,
    /// HTML já escapado pelo tantivy, com os termos em `<b>`.
    pub snippet: String,
    pub score: f32,
}

pub struct SearchOpts {
    pub query: String,
    pub feed_id: Option<i64>,
    pub unread_only: bool,
    pub favorites_only: bool,
    /// Só artigos publicados (ou baixados) a partir deste instante.
    pub since_ms: Option<i64>,
    pub limit: usize,
}

/// Abre o índice em `dir`, criando do zero se não existir, se o schema mudou
/// ou se o que está lá não abre. **Nunca falha por índice corrompido**: o
/// índice é derivado do SQLite, então apagar e recomeçar é sempre seguro.
pub fn open_or_create(dir: &Path) -> Result<FtIndex, String> {
    if !schema_matches(dir) {
        let _ = fs::remove_dir_all(dir);
    }
    match try_open(dir) {
        Ok(ft) => Ok(ft),
        Err(_) => {
            let _ = fs::remove_dir_all(dir);
            try_open(dir)
        }
    }
}

fn schema_matches(dir: &Path) -> bool {
    let Ok(txt) = fs::read_to_string(dir.join(META_FILE)) else {
        // Sem marcador: ou é pasta nova (o try_open cria) ou é lixo de uma
        // versão anterior — nos dois casos recomeçar é o certo.
        return !dir.exists();
    };
    serde_json::from_str::<serde_json::Value>(&txt)
        .ok()
        .and_then(|v| v.get("schema").and_then(|s| s.as_u64()))
        == Some(SCHEMA_VERSION as u64)
}

fn try_open(dir: &Path) -> Result<FtIndex, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let schema = build_schema();
    let index = match Index::open_in_dir(dir) {
        Ok(i) => i,
        Err(_) => Index::create_in_dir(dir, schema.clone()).map_err(|e| e.to_string())?,
    };
    index.tokenizers().register(TOKENIZER, folding_analyzer());
    let s = index.schema();
    let get = |n: &str| s.get_field(n).map_err(|e| e.to_string());
    let ft = FtIndex {
        aid_f: get("aid")?,
        title_f: get("title")?,
        body_f: get("body")?,
        author_f: get("author")?,
        feed_f: get("feed")?,
        writer: index
            .writer_with_num_threads(1, WRITER_HEAP)
            .map_err(|e| e.to_string())?,
        // Manual: quem commita chama `reload()` na sequência. Com a política
        // por tempo, uma busca logo depois de indexar podia não ver o artigo
        // que acabou de chegar — aqui a ordem é determinística.
        reader: index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|e: tantivy::TantivyError| e.to_string())?,
        index,
    };
    fs::write(
        dir.join(META_FILE),
        format!("{{\"schema\":{SCHEMA_VERSION}}}"),
    )
    .map_err(|e| e.to_string())?;
    Ok(ft)
}

impl FtIndex {
    /// Commit + reload do reader.
    ///
    /// A retentativa não é paranoia: no Windows o merge em segundo plano
    /// apaga segmentos que o SO ainda tem mapeados e o commit volta com
    /// "Acesso negado" (os error 5). É intermitente e aparece justamente no
    /// backfill, que é a sequência mais longa de commits que o app faz.
    /// Perder o commit significaria artigo fora do índice até o próximo boot.
    pub fn commit(&mut self) -> Result<(), String> {
        let mut last = String::new();
        for tentativa in 0..3 {
            match self.writer.commit() {
                Ok(_) => {
                    self.reader.reload().map_err(|e| e.to_string())?;
                    return Ok(());
                }
                Err(e) => {
                    last = e.to_string();
                    std::thread::sleep(std::time::Duration::from_millis(120 * (tentativa + 1)));
                }
            }
        }
        Err(last)
    }

    /// Insere (ou substitui) os documentos SEM commitar — o backfill enfileira
    /// vários lotes por commit (menos commit = menos merge = menos corrida no
    /// Windows). Quem chama é obrigado a fechar com `commit`.
    pub fn add_docs(&mut self, docs: &[IndexDoc]) -> Result<(), String> {
        for d in docs {
            let aid = d.id as u64;
            // Delete antes do add: reindexar o mesmo artigo (ex.: o texto
            // completo chegou pelo readability) não pode duplicar.
            self.writer.delete_term(Term::from_field_u64(self.aid_f, aid));
            self.writer
                .add_document(doc!(
                    self.aid_f => aid,
                    self.title_f => d.title.clone(),
                    self.body_f => d.body.clone(),
                    self.author_f => d.author.clone(),
                    self.feed_f => d.feed.clone(),
                ))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Atalho pro caminho incremental (poucos artigos chegando): indexa e
    /// commita de uma vez, pra ficarem buscáveis já.
    pub fn index_docs(&mut self, docs: &[IndexDoc]) -> Result<(), String> {
        if docs.is_empty() {
            return Ok(());
        }
        self.add_docs(docs)?;
        self.commit()
    }

    pub fn delete_ids(&mut self, ids: &[i64]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        for id in ids {
            self.writer
                .delete_term(Term::from_field_u64(self.aid_f, *id as u64));
        }
        self.commit()
    }

    /// Todos os ids presentes no índice (lê a coluna fast, sem tocar em
    /// documento armazenado — barato mesmo com centenas de milhares).
    pub fn indexed_ids(&self) -> Result<HashSet<i64>, String> {
        let searcher = self.reader.searcher();
        let mut out = HashSet::new();
        for seg in searcher.segment_readers() {
            let col = seg.fast_fields().u64("aid").map_err(|e| e.to_string())?;
            for doc in seg.doc_ids_alive() {
                if let Some(v) = col.first(doc) {
                    out.insert(v as i64);
                }
            }
        }
        Ok(out)
    }

    pub fn doc_count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }
}

/// Soma dos arquivos da pasta do índice (pro painel de armazenamento).
pub fn index_bytes(dir: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

// ---------- busca ----------

/// Busca no índice e resolve cada acerto no SQLite.
///
/// O tantivy só ordena por relevância; **os filtros (feed, período, não-lidos,
/// favoritos) são do SQLite**, porque lido/favorito mudam sem parar e não
/// podem morar num índice. Por isso pedimos mais candidatos do que o limite:
/// os que não passam no filtro são descartados na resolução.
pub fn search(conn: &Connection, ft: &FtIndex, o: &SearchOpts) -> Result<Vec<SearchHit>, String> {
    let limit = o.limit.clamp(1, 200);
    let filtered = o.feed_id.is_some() || o.unread_only || o.favorites_only || o.since_ms.is_some();
    let candidates = if filtered { (limit * 8).min(1000) } else { limit };

    let searcher = ft.reader.searcher();
    let mut parser = QueryParser::for_index(
        &ft.index,
        vec![ft.title_f, ft.body_f, ft.author_f, ft.feed_f],
    );
    parser.set_field_boost(ft.title_f, 3.0);
    parser.set_field_boost(ft.feed_f, 0.5);
    let (query, _errs) = parser.parse_query_lenient(&o.query);
    let top = searcher
        .search(&*query, &TopDocs::with_limit(candidates))
        .map_err(|e| e.to_string())?;

    let mut sg =
        SnippetGenerator::create(&searcher, &*query, ft.body_f).map_err(|e| e.to_string())?;
    sg.set_max_num_chars(220);

    // Um SELECT por acerto: o filtro vai no WHERE, então o artigo que não
    // passa simplesmente não devolve linha.
    let mut sql = String::from(
        "SELECT a.id, a.feed_id, f.title, a.title, a.url, a.author, a.published_ms,
                a.excerpt, a.read, a.favorite, COALESCE(a.content, a.summary, a.excerpt)
         FROM articles a JOIN feeds f ON f.id = a.feed_id
         WHERE a.id = ?1",
    );
    if let Some(id) = o.feed_id {
        sql.push_str(&format!(" AND a.feed_id = {id}"));
    }
    if o.unread_only {
        sql.push_str(" AND a.read = 0");
    }
    if o.favorites_only {
        sql.push_str(" AND a.favorite = 1");
    }
    if let Some(ms) = o.since_ms {
        sql.push_str(&format!(" AND COALESCE(a.published_ms, a.fetched_ms) >= {ms}"));
    }
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let mut out: Vec<SearchHit> = Vec::with_capacity(limit);
    for (score, addr) in top {
        if out.len() >= limit {
            break;
        }
        let stored = searcher.segment_reader(addr.segment_ord);
        let col = stored.fast_fields().u64("aid").map_err(|e| e.to_string())?;
        let Some(aid) = col.first(addr.doc_id) else {
            continue;
        };
        let row = stmt.query_row([aid as i64], |r| {
            Ok((
                ArticleRow {
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
                },
                r.get::<_, Option<String>>(10)?.unwrap_or_default(),
            ))
        });
        // Sem linha = filtrado, ou artigo apagado que o índice ainda não
        // esqueceu (o reconcile limpa no próximo boot). Nos dois casos: pula.
        let Ok((article, raw)) = row else { continue };
        let text = html_to_text(raw.as_bytes());
        let snippet = sg.snippet(&text).to_html();
        out.push(SearchHit {
            article,
            snippet,
            score,
        });
    }
    Ok(out)
}

// ---------- alimentação do índice ----------

/// Lê do SQLite os artigos pedidos, já com o texto extraído do HTML.
pub fn fetch_docs(conn: &Connection, ids: &[i64]) -> Result<Vec<IndexDoc>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.title, a.author, f.title,
                    COALESCE(a.content, a.summary, a.excerpt)
             FROM articles a JOIN feeds f ON f.id = a.feed_id
             WHERE a.id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let row = stmt.query_row([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        });
        if let Ok((id, title, author, feed, body)) = row {
            out.push(IndexDoc {
                id,
                title,
                author: author.unwrap_or_default(),
                feed,
                body: html_to_text(body.unwrap_or_default().as_bytes()),
            });
        }
    }
    Ok(out)
}

/// Ids de artigo que existem no SQLite mas não no índice, e vice-versa.
///
/// É o que torna o índice descartável: em vez de confiar numa marca d'água
/// (id máximo indexado — que o SQLite quebra ao reaproveitar rowid de linha
/// apagada), comparamos os dois conjuntos. Custa um HashSet de u64 e resolve
/// tanto a primeira execução de quem já tem artigos quanto índice truncado.
pub fn reconcile_plan(conn: &Connection, ft: &FtIndex) -> Result<(Vec<i64>, Vec<i64>), String> {
    let indexed = ft.indexed_ids()?;
    let mut stmt = conn
        .prepare("SELECT id FROM articles")
        .map_err(|e| e.to_string())?;
    let mut in_db: HashSet<i64> = HashSet::new();
    let rows = stmt
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    for r in rows {
        in_db.insert(r.map_err(|e| e.to_string())?);
    }
    let missing: Vec<i64> = in_db.difference(&indexed).copied().collect();
    let orphans: Vec<i64> = indexed.difference(&in_db).copied().collect();
    Ok((missing, orphans))
}

/// Extração de texto simples: descarta script/style, tags e entidades comuns.
/// Cópia fiel do LocalZIM — o `strip_html` do fetch.rs corta em 500 chars
/// (serve pro excerpt, não pro índice).
pub fn html_to_text(html: &[u8]) -> String {
    let s = String::from_utf8_lossy(html);
    // ASCII lowercase preserva os índices de byte do original
    let low = s.to_ascii_lowercase();
    let src = s.as_bytes();
    let n = src.len();
    let mut out: Vec<u8> = Vec::with_capacity(n / 3);
    let mut i = 0usize;
    while i < n {
        if src[i] == b'<' {
            if low[i..].starts_with("<script") || low[i..].starts_with("<style") {
                let close = if low[i..].starts_with("<script") {
                    "</script"
                } else {
                    "</style"
                };
                i = low[i..].find(close).map(|p| i + p).unwrap_or(n);
            }
            i = low[i..].find('>').map(|p| i + p + 1).unwrap_or(n);
            out.push(b' ');
        } else {
            out.push(src[i]);
            i += 1;
        }
    }
    let text = String::from_utf8_lossy(&out)
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SEQ: AtomicU32 = AtomicU32::new(0);

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!(
            "localfeed-ft-{tag}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn db_com_artigos(artigos: &[(&str, &str)]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO feeds (id, url, title, added_ms) VALUES (1, 'https://ex.com/f', 'Blog do Ex', 0)",
            [],
        )
        .unwrap();
        for (i, (titulo, corpo)) in artigos.iter().enumerate() {
            conn.execute(
                "INSERT INTO articles (feed_id, guid, title, author, published_ms, excerpt, content, fetched_ms)
                 VALUES (1, ?1, ?2, 'Maria Silva', ?3, '', ?4, ?3)",
                rusqlite::params![format!("g{i}"), titulo, (i as i64 + 1) * 1000, corpo],
            )
            .unwrap();
        }
        conn
    }

    fn indexa_tudo(conn: &Connection, ft: &mut FtIndex) {
        let (missing, orphans) = reconcile_plan(conn, ft).unwrap();
        ft.delete_ids(&orphans).unwrap();
        let docs = fetch_docs(conn, &missing).unwrap();
        ft.index_docs(&docs).unwrap();
    }

    fn ids(hits: &[SearchHit]) -> Vec<i64> {
        hits.iter().map(|h| h.article.id).collect()
    }

    fn opts(q: &str) -> SearchOpts {
        SearchOpts {
            query: q.into(),
            feed_id: None,
            unread_only: false,
            favorites_only: false,
            since_ms: None,
            limit: 20,
        }
    }

    #[test]
    fn html_to_text_descarta_script_e_style() {
        let html = b"<p>Texto <b>bom</b> &amp; util</p><style>p{color:red}</style>\
                     <script>var x = 'segredo';</script>";
        let t = html_to_text(html);
        assert!(t.contains("Texto bom & util"));
        assert!(!t.contains("color"));
        assert!(!t.contains("segredo"));
    }

    #[test]
    fn acha_o_que_deve_e_nao_acha_o_que_nao_deve() {
        let conn = db_com_artigos(&[
            ("Receita de pão de queijo", "<p>Polvilho azedo, queijo minas e ovo.</p>"),
            ("Como trocar o pneu", "<p>Macaco hidráulico e chave de roda.</p>"),
            ("São Paulo tem novo metrô", "<p>A linha liga a zona leste ao centro.</p>"),
        ]);
        let dir = tmp_dir("basico");
        let mut ft = open_or_create(&dir).unwrap();
        indexa_tudo(&conn, &mut ft);
        assert_eq!(ft.doc_count(), 3);

        // acha pelo título
        assert_eq!(ids(&search(&conn, &ft, &opts("queijo")).unwrap()), vec![1]);
        // acha pelo corpo (palavra que NÃO está no título)
        assert_eq!(ids(&search(&conn, &ft, &opts("polvilho")).unwrap()), vec![1]);
        assert_eq!(ids(&search(&conn, &ft, &opts("hidraulico")).unwrap()), vec![2]);
        // sem acento acha com acento (ascii folding), nos dois sentidos
        assert_eq!(ids(&search(&conn, &ft, &opts("metro")).unwrap()), vec![3]);
        assert_eq!(ids(&search(&conn, &ft, &opts("sáo páulo")).unwrap()), vec![3]);
        // acha pelo autor e pelo feed
        assert_eq!(search(&conn, &ft, &opts("Maria Silva")).unwrap().len(), 3);
        assert_eq!(search(&conn, &ft, &opts("Blog do Ex")).unwrap().len(), 3);

        // FALSO POSITIVO: termo que não está em lugar nenhum não pode voltar
        assert!(search(&conn, &ft, &opts("bicicleta")).unwrap().is_empty());
        // …nem pedaço de palavra (é busca por termo, não substring)
        assert!(search(&conn, &ft, &opts("queij")).unwrap().is_empty());
        // …nem o que só existe DENTRO de tag HTML
        assert!(search(&conn, &ft, &opts("p")).unwrap().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippet_destaca_o_termo() {
        let conn = db_com_artigos(&[(
            "Título qualquer",
            "<p>Uma frase longa qualquer só pra ter contexto ao redor da palavra \
             polvilho, que é a que buscamos aqui no meio do texto.</p>",
        )]);
        let dir = tmp_dir("snippet");
        let mut ft = open_or_create(&dir).unwrap();
        indexa_tudo(&conn, &mut ft);

        let hits = search(&conn, &ft, &opts("polvilho")).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains("<b>polvilho</b>"), "{}", hits[0].snippet);
        // o trecho traz contexto, não a palavra solta
        assert!(hits[0].snippet.contains("contexto"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn filtros_por_feed_periodo_e_nao_lidos() {
        let conn = db_com_artigos(&[
            ("Pão A", "<p>polvilho</p>"),
            ("Pão B", "<p>polvilho</p>"),
        ]);
        // segundo feed com um artigo que também bate
        conn.execute(
            "INSERT INTO feeds (id, url, title, added_ms) VALUES (2, 'https://o.com/f', 'Outro', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO articles (feed_id, guid, title, published_ms, excerpt, content, fetched_ms, read)
             VALUES (2, 'x', 'Pão C', 9000, '', '<p>polvilho</p>', 9000, 1)",
            [],
        )
        .unwrap();
        let dir = tmp_dir("filtros");
        let mut ft = open_or_create(&dir).unwrap();
        indexa_tudo(&conn, &mut ft);

        assert_eq!(search(&conn, &ft, &opts("polvilho")).unwrap().len(), 3);

        let mut o = opts("polvilho");
        o.feed_id = Some(2);
        assert_eq!(ids(&search(&conn, &ft, &o).unwrap()), vec![3]);

        let mut o = opts("polvilho");
        o.unread_only = true; // o id 3 está lido
        let mut got = ids(&search(&conn, &ft, &o).unwrap());
        got.sort();
        assert_eq!(got, vec![1, 2]);

        let mut o = opts("polvilho");
        o.since_ms = Some(5000); // só o id 3 é "recente"
        assert_eq!(ids(&search(&conn, &ft, &o).unwrap()), vec![3]);

        let mut o = opts("polvilho");
        o.favorites_only = true; // nenhum é favorito
        assert!(search(&conn, &ft, &o).unwrap().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn incremental_nao_duplica_e_apaga_some_do_resultado() {
        let conn = db_com_artigos(&[("Um", "<p>alfa</p>")]);
        let dir = tmp_dir("incremental");
        let mut ft = open_or_create(&dir).unwrap();
        indexa_tudo(&conn, &mut ft);

        // artigo novo entra sozinho, sem reindexar o que já estava
        conn.execute(
            "INSERT INTO articles (feed_id, guid, title, published_ms, excerpt, content, fetched_ms)
             VALUES (1, 'novo', 'Dois', 2000, '', '<p>alfa beta</p>', 2000)",
            [],
        )
        .unwrap();
        let docs = fetch_docs(&conn, &[2]).unwrap();
        ft.index_docs(&docs).unwrap();
        assert_eq!(ft.doc_count(), 2);
        assert_eq!(search(&conn, &ft, &opts("alfa")).unwrap().len(), 2);

        // reindexar o MESMO artigo (texto completo chegou depois) não duplica
        let docs = fetch_docs(&conn, &[2]).unwrap();
        ft.index_docs(&docs).unwrap();
        assert_eq!(ft.doc_count(), 2);
        assert_eq!(search(&conn, &ft, &opts("beta")).unwrap().len(), 1);

        // apagado do banco some do resultado mesmo antes do reconcile
        conn.execute("DELETE FROM articles WHERE id = 2", []).unwrap();
        assert_eq!(ids(&search(&conn, &ft, &opts("alfa")).unwrap()), vec![1]);
        // …e o reconcile identifica o órfão
        let (missing, orphans) = reconcile_plan(&conn, &ft).unwrap();
        assert!(missing.is_empty());
        assert_eq!(orphans, vec![2]);
        ft.delete_ids(&orphans).unwrap();
        assert_eq!(ft.doc_count(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn indice_corrompido_e_recriado_sem_perder_artigo() {
        let conn = db_com_artigos(&[("Um", "<p>alfa</p>")]);
        let dir = tmp_dir("corrompido");
        {
            let mut ft = open_or_create(&dir).unwrap();
            indexa_tudo(&conn, &mut ft);
            assert_eq!(ft.doc_count(), 1);
        }
        // simula corrupção: mete lixo por cima do meta do tantivy
        fs::write(dir.join("meta.json"), b"nao sou json").unwrap();

        let mut ft = open_or_create(&dir).unwrap();
        assert_eq!(ft.doc_count(), 0, "índice recriado vazio");
        // o artigo continua no SQLite e o reconcile o traz de volta
        let (missing, _) = reconcile_plan(&conn, &ft).unwrap();
        assert_eq!(missing, vec![1]);
        indexa_tudo(&conn, &mut ft);
        assert_eq!(search(&conn, &ft, &opts("alfa")).unwrap().len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    /// O caminho da ATUALIZAÇÃO, que é o que morde: quem já usava o LocalFeed
    /// chega na versão nova com banco cheio e ZERO índice. Aqui o banco é um
    /// arquivo de verdade, aberto pelo `db::open` (migrações inclusive), pra
    /// garantir que a ordem do boot está certa — índice DEPOIS do schema.
    #[test]
    fn caminho_da_atualizacao_indexa_quem_ja_tinha_artigos() {
        let base = tmp_dir("upgrade");
        fs::create_dir_all(&base).unwrap();
        let db_path = base.join("localfeed.db");

        // --- versão antiga: usuário acumulou artigos, sem índice nenhum ---
        {
            let conn = crate::db::open(&db_path).unwrap();
            conn.execute(
                "INSERT INTO feeds (id, url, title, added_ms) VALUES (1, 'u', 'Feed Velho', 0)",
                [],
            )
            .unwrap();
            for i in 0..50 {
                conn.execute(
                    "INSERT INTO articles (feed_id, guid, title, published_ms, excerpt, content, fetched_ms)
                     VALUES (1, ?1, ?2, ?3, '', ?4, ?3)",
                    rusqlite::params![
                        format!("g{i}"),
                        format!("Artigo antigo {i}"),
                        i as i64 * 1000,
                        format!("<p>conteudo velho marca{i}</p>")
                    ],
                )
                .unwrap();
            }
        }
        let idx = index_dir(&base);
        assert!(!idx.exists(), "usuário da versão antiga não tem índice");

        // --- primeiro boot da versão nova ---
        let conn = crate::db::open(&db_path).unwrap();
        let mut ft = open_or_create(&idx).unwrap();
        let (missing, orphans) = reconcile_plan(&conn, &ft).unwrap();
        assert_eq!(missing.len(), 50, "todos os artigos antigos entram no índice");
        assert!(orphans.is_empty());

        // backfill em lotes, como o spawn_reconcile faz
        for chunk in missing.chunks(20) {
            let docs = fetch_docs(&conn, chunk).unwrap();
            ft.add_docs(&docs).unwrap();
        }
        ft.commit().unwrap();
        assert_eq!(ft.doc_count(), 50);
        assert_eq!(search(&conn, &ft, &opts("marca7")).unwrap().len(), 1);

        // --- segundo boot: nada a fazer, não reindexa o que já está lá ---
        drop(ft);
        let ft = open_or_create(&idx).unwrap();
        let (missing, orphans) = reconcile_plan(&conn, &ft).unwrap();
        assert!(missing.is_empty(), "boot seguinte não reindexa nada");
        assert!(orphans.is_empty());
        assert_eq!(search(&conn, &ft, &opts("marca7")).unwrap().len(), 1);

        let _ = fs::remove_dir_all(&base);
    }

    /// App fechado no meio do backfill: o que faltou entra no boot seguinte,
    /// e só o que faltou (a marca d'água por id não daria essa garantia).
    #[test]
    fn backfill_interrompido_retoma_de_onde_parou() {
        let conn = db_com_artigos(&[
            ("Um", "<p>alfa</p>"),
            ("Dois", "<p>beta</p>"),
            ("Tres", "<p>gama</p>"),
            ("Quatro", "<p>delta</p>"),
        ]);
        let dir = tmp_dir("interrompido");
        {
            let mut ft = open_or_create(&dir).unwrap();
            // só metade indexada — e então o app "morre"
            let docs = fetch_docs(&conn, &[1, 2]).unwrap();
            ft.index_docs(&docs).unwrap();
        }

        let mut ft = open_or_create(&dir).unwrap();
        let (mut missing, _) = reconcile_plan(&conn, &ft).unwrap();
        missing.sort();
        assert_eq!(missing, vec![3, 4], "só o que ficou faltando");

        let docs = fetch_docs(&conn, &missing).unwrap();
        ft.index_docs(&docs).unwrap();
        assert_eq!(ft.doc_count(), 4);
        for termo in ["alfa", "beta", "gama", "delta"] {
            assert_eq!(search(&conn, &ft, &opts(termo)).unwrap().len(), 1, "{termo}");
        }

        let _ = fs::remove_dir_all(&dir);
    }

    /// Volume realista: mede tamanho do índice e tempo de busca.
    /// `cargo test --release volume_realista -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn volume_realista() {
        const N: usize = 20_000;
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO feeds (id, url, title, added_ms) VALUES (1, 'u', 'Feed', 0)",
            [],
        )
        .unwrap();
        // ~3 KB de texto por artigo (artigo de blog médio)
        let lorem: String = std::iter::repeat(
            "o gato subiu no telhado e ninguem sabe como desceu porque a escada sumiu ",
        )
        .take(42)
        .collect();
        for i in 0..N {
            conn.execute(
                "INSERT INTO articles (feed_id, guid, title, published_ms, excerpt, content, fetched_ms)
                 VALUES (1, ?1, ?2, ?3, '', ?4, ?3)",
                rusqlite::params![
                    format!("g{i}"),
                    format!("Artigo numero {i} sobre coisas"),
                    i as i64 * 1000,
                    format!("<p>{lorem} marcador{i} fim.</p>")
                ],
            )
            .unwrap();
        }
        let dir = tmp_dir("volume");
        let mut ft = open_or_create(&dir).unwrap();

        let t0 = std::time::Instant::now();
        let (missing, _) = reconcile_plan(&conn, &ft).unwrap();
        // Mesmo ritmo do backfill real: lotes de 200, commit a cada 2000.
        for (i, chunk) in missing.chunks(200).enumerate() {
            let docs = fetch_docs(&conn, chunk).unwrap();
            ft.add_docs(&docs).unwrap();
            if (i + 1) % 10 == 0 {
                ft.commit().unwrap();
            }
        }
        ft.commit().unwrap();
        let idx_ms = t0.elapsed().as_millis();

        let bytes = index_bytes(&dir);
        println!(
            "VOLUME: {N} artigos | indexação {idx_ms} ms | índice {:.1} MB ({} bytes)",
            bytes as f64 / 1_048_576.0,
            bytes
        );

        for q in ["telhado", "marcador19999", "gato escada", "artigo numero 500"] {
            let t = std::time::Instant::now();
            let hits = search(&conn, &ft, &opts(q)).unwrap();
            println!(
                "BUSCA {q:?}: {} acertos em {:.2} ms",
                hits.len(),
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        // com filtro (over-fetch de candidatos, o pior caso)
        let mut o = opts("telhado");
        o.unread_only = true;
        let t = std::time::Instant::now();
        let hits = search(&conn, &ft, &o).unwrap();
        println!(
            "BUSCA filtrada: {} acertos em {:.2} ms",
            hits.len(),
            t.elapsed().as_secs_f64() * 1000.0
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
