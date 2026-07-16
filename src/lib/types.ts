/** Espelho dos structs do Rust (serde camelCase). */

export interface FeedRow {
  id: number;
  url: string;
  title: string;
  siteUrl: string | null;
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
