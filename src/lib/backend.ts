import { invoke } from "@tauri-apps/api/core";
import type { ArticleFull, ArticleRow, FeedRow, ListFilter, OpmlImport, RefreshSummary } from "./types";

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

export function refreshAll(): Promise<RefreshSummary> {
  return invoke("refresh_all");
}

export function listArticles(filter: ListFilter): Promise<ArticleRow[]> {
  return invoke("list_articles", {
    feedId: filter.kind === "feed" ? filter.feedId : null,
    unreadOnly: filter.kind === "unread",
    favoritesOnly: filter.kind === "favorites",
  });
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

export function importOpml(path: string): Promise<OpmlImport> {
  return invoke("import_opml", { path });
}

export function exportOpml(path: string): Promise<void> {
  return invoke("export_opml", { path });
}

export function getStartupFile(): Promise<string | null> {
  return invoke("get_startup_file");
}
