/** Espelho dos structs do Rust (serde camelCase). */

export interface FeedRow {
  id: number;
  url: string;
  title: string;
  siteUrl: string | null;
  folder: string | null;
  unread: number;
  lastError: string | null;
}

export interface ArticleRow {
  id: number;
  feedId: number;
  feedTitle: string;
  title: string;
  url: string | null;
  author: string | null;
  publishedMs: number | null;
  excerpt: string;
  read: boolean;
  favorite: boolean;
}

export interface ArticleFull {
  id: number;
  feedId: number;
  feedTitle: string;
  title: string;
  url: string | null;
  author: string | null;
  publishedMs: number | null;
  contentHtml: string | null;
  read: boolean;
  favorite: boolean;
}

/** Período da busca full-text. */
export type SearchPeriod = "any" | "week" | "month" | "year";

/** Um acerto da busca full-text (espelho do SearchHit do Rust). */
export interface SearchHit {
  article: ArticleRow;
  /** HTML já escapado pelo tantivy, com os termos em `<b>`. */
  snippet: string;
  score: number;
}

/** Estado do índice de busca (espelho do SearchStatus do Rust). */
export interface SearchStatus {
  /** Backfill inicial rodando — quem já tinha artigos ganha o índice agora. */
  building: boolean;
  done: number;
  total: number;
  docs: number;
}

export interface RefreshSummary {
  newArticles: number;
  errors: string[];
}

export interface OpmlImport {
  added: number;
  skipped: number;
}

/** Filtro da lista: tudo, não lidos, favoritos ou um feed específico. */
export type ListFilter =
  | { kind: "all" }
  | { kind: "unread" }
  | { kind: "favorites" }
  | { kind: "feed"; feedId: number };

/** Painel "Dados e armazenamento" (espelho do StorageInfo do Rust). */
export interface StorageInfo {
  /** Pasta de dados do app (onde mora o localfeed.db). */
  dir: string;
  /** Tamanho do banco em bytes (db + WAL + SHM). */
  dbBytes: number;
  /** Tamanho do índice de busca em bytes (derivado — reconstrói sozinho). */
  indexBytes: number;
  articles: number;
  cached: number;
  favorites: number;
}
