import { invoke } from "@tauri-apps/api/core";
import type {
  ArticleFull,
  ArticleRow,
  FeedRow,
  ListFilter,
  OpmlImport,
  RefreshSummary,
  SearchHit,
  SearchStatus,
  StorageInfo,
} from "./types";

/** Rodando dentro do Tauri? (o smoke em navegador puro não tem a ponte.) */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function listFeeds(): Promise<FeedRow[]> {
  return invoke("list_feeds");
}

export function addFeed(url: string): Promise<FeedRow> {
  return invoke("add_feed", { url });
}

export function removeFeed(feedId: number): Promise<void> {
  return invoke("remove_feed", { feedId });
}

export function setFeedFolder(feedId: number, folder: string | null): Promise<void> {
  return invoke("set_feed_folder", { feedId, folder });
}

export function refreshAll(): Promise<RefreshSummary> {
  return invoke("refresh_all");
}

export function listArticles(filter: ListFilter): Promise<ArticleRow[]> {
  return invoke("list_articles", {
    feedId: filter.kind === "feed" ? filter.feedId : null,
    unreadOnly: filter.kind === "unread",
    favoritesOnly: filter.kind === "favorites",
    laterOnly: filter.kind === "later",
  });
}

/**
 * Busca full-text no índice tantivy. Os filtros vêm do mesmo `ListFilter` da
 * lista — buscar dentro de um feed é o filtro da barra lateral, não um
 * controle novo.
 */
export function searchArticles(
  query: string,
  filter: ListFilter,
  sinceMs: number | null,
): Promise<SearchHit[]> {
  return invoke("search_articles", {
    query,
    feedId: filter.kind === "feed" ? filter.feedId : null,
    unreadOnly: filter.kind === "unread",
    favoritesOnly: filter.kind === "favorites",
    laterOnly: filter.kind === "later",
    sinceMs,
    limit: 100,
  });
}

export function searchStatus(): Promise<SearchStatus> {
  return invoke("search_status");
}

export function getArticle(articleId: number): Promise<ArticleFull> {
  return invoke("get_article", { articleId });
}

export function markRead(articleId: number, read: boolean): Promise<void> {
  return invoke("mark_read", { articleId, read });
}

export function markAllRead(feedId: number | null): Promise<void> {
  return invoke("mark_all_read", { feedId });
}

export function toggleFavorite(articleId: number): Promise<boolean> {
  return invoke("toggle_favorite", { articleId });
}

export function toggleLater(articleId: number): Promise<boolean> {
  return invoke("toggle_later", { articleId });
}

export function importOpml(path: string): Promise<OpmlImport> {
  return invoke("import_opml", { path });
}

export function exportOpml(path: string): Promise<void> {
  return invoke("export_opml", { path });
}

export function getStartupFile(): Promise<string | null> {
  return invoke("get_startup_file");
}

export function storageInfo(): Promise<StorageInfo> {
  return invoke("storage_info");
}

/** Limpa só o conteúdo readability em cache; retorna quantos artigos limpou. */
export function clearReadabilityCache(): Promise<number> {
  return invoke("clear_readability_cache");
}

/** Apaga artigos não favoritos com mais de N dias; retorna quantos apagou. */
export function clearOldArticles(days: number): Promise<number> {
  return invoke("clear_old_articles", { days });
}
