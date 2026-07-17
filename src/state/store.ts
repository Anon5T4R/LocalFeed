import { create } from "zustand";
import * as backend from "../lib/backend";
import { t } from "../lib/i18n";
import type { ArticleFull, ArticleRow, FeedRow, ListFilter } from "../lib/types";
import { useUi } from "./ui";

interface FeedState {
  feeds: FeedRow[];
  filter: ListFilter;
  articles: ArticleRow[];
  /** Busca ao vivo (filtra a lista por título/resumo). */
  query: string;
  listLoading: boolean;
  current: ArticleFull | null;
  refreshing: boolean;

  loadFeeds: () => Promise<void>;
  setFilter: (f: ListFilter) => Promise<void>;
  setQuery: (q: string) => void;
  reloadArticles: () => Promise<void>;
  openArticle: (id: number) => Promise<void>;
  addFeed: (url: string) => Promise<boolean>;
  removeFeed: (feedId: number) => Promise<void>;
  moveFeed: (feedId: number, folder: string | null) => Promise<void>;
  refreshAll: () => Promise<void>;
  markAllRead: () => Promise<void>;
  toggleFavorite: () => Promise<void>;
  markUnread: () => Promise<void>;
  /** Alterna lido/não-lido de um artigo da lista (navegação por teclado: m). */
  toggleReadArticle: (id: number) => Promise<void>;
  /** Alterna favorito de um artigo da lista (navegação por teclado: s). */
  toggleFavoriteArticle: (id: number) => Promise<void>;
}

export const useFeed = create<FeedState>((set, get) => ({
  feeds: [],
  filter: { kind: "all" },
  articles: [],
  query: "",
  listLoading: false,
  current: null,
  refreshing: false,

  loadFeeds: async () => {
    const feeds = await backend.listFeeds().catch(() => [] as FeedRow[]);
    set({ feeds });
  },

  setQuery: (query) => set({ query }),

  setFilter: async (filter) => {
    set({ filter, current: null });
    await get().reloadArticles();
  },

  reloadArticles: async () => {
    set({ listLoading: true });
    try {
      const articles = await backend.listArticles(get().filter);
      set({ articles, listLoading: false });
    } catch {
      set({ articles: [], listLoading: false });
    }
  },

  openArticle: async (id) => {
    // Marca lido já na abertura (otimista) e busca o conteúdo completo.
    const s = get();
    set({
      articles: s.articles.map((a) => (a.id === id ? { ...a, read: true } : a)),
    });
    void backend.markRead(id, true).catch(() => {});
    try {
      const current = await backend.getArticle(id);
      set({ current: { ...current, read: true } });
      void get().loadFeeds(); // badges de não-lidos
    } catch (e) {
      useUi.getState().pushToast("error", String(e));
    }
  },

  addFeed: async (url) => {
    const ui = useUi.getState();
    try {
      const feed = await backend.addFeed(url);
      ui.pushToast("ok", t("toast.added", { title: feed.title }));
      await get().loadFeeds();
      await get().reloadArticles();
      return true;
    } catch (e) {
      ui.pushToast("error", t("toast.addFailed", { error: String(e) }));
      return false;
    }
  },

  removeFeed: async (feedId) => {
    await backend.removeFeed(feedId).catch(() => {});
    useUi.getState().pushToast("info", t("toast.removed"));
    const s = get();
    if (s.filter.kind === "feed" && s.filter.feedId === feedId) {
      set({ filter: { kind: "all" }, current: null });
    }
    await s.loadFeeds();
    await get().reloadArticles();
  },

  moveFeed: async (feedId, folder) => {
    await backend.setFeedFolder(feedId, folder).catch(() => {});
    await get().loadFeeds();
  },

  toggleReadArticle: async (id) => {
    const a = get().articles.find((x) => x.id === id);
    if (!a) return;
    const read = !a.read;
    await backend.markRead(id, read).catch(() => {});
    set({ articles: get().articles.map((x) => (x.id === id ? { ...x, read } : x)) });
    void get().loadFeeds();
  },

  toggleFavoriteArticle: async (id) => {
    const favorite = await backend.toggleFavorite(id).catch(() => undefined);
    if (favorite === undefined) return;
    const cur = get().current;
    set({
      articles: get().articles.map((x) => (x.id === id ? { ...x, favorite } : x)),
      current: cur && cur.id === id ? { ...cur, favorite } : cur,
    });
  },

  refreshAll: async () => {
    if (get().refreshing) return;
    set({ refreshing: true });
    const ui = useUi.getState();
    try {
      const summary = await backend.refreshAll();
      if (summary.errors.length > 0) {
        ui.pushToast("error", t("toast.refreshErrors", {
          n: summary.errors.length,
          first: summary.errors[0],
        }));
      }
      ui.pushToast(
        summary.newArticles > 0 ? "ok" : "info",
        summary.newArticles > 0
          ? t("toast.refreshDone", { n: summary.newArticles })
          : t("toast.refreshNone"),
      );
      await get().loadFeeds();
      await get().reloadArticles();
    } catch (e) {
      ui.pushToast("error", String(e));
    } finally {
      set({ refreshing: false });
    }
  },

  markAllRead: async () => {
    const f = get().filter;
    await backend.markAllRead(f.kind === "feed" ? f.feedId : null).catch(() => {});
    await get().loadFeeds();
    await get().reloadArticles();
  },

  toggleFavorite: async () => {
    const cur = get().current;
    if (!cur) return;
    const favorite = await backend.toggleFavorite(cur.id).catch(() => cur.favorite);
    set({
      current: { ...cur, favorite },
      articles: get().articles.map((a) => (a.id === cur.id ? { ...a, favorite } : a)),
    });
  },

  markUnread: async () => {
    const cur = get().current;
    if (!cur) return;
    await backend.markRead(cur.id, false).catch(() => {});
    set({
      current: null,
      articles: get().articles.map((a) => (a.id === cur.id ? { ...a, read: false } : a)),
    });
    void get().loadFeeds();
  },
}));
