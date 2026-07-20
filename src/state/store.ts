import { create } from "zustand";
import * as backend from "../lib/backend";
import { t } from "../lib/i18n";
import { isSearchable, sinceMsFor } from "../lib/search";
import type {
  ArticleFull,
  ArticleRow,
  FeedRow,
  ListFilter,
  SearchHit,
  SearchPeriod,
} from "../lib/types";
import { useUi } from "./ui";

/**
 * Lido/favorito mudam nos dois lugares: a lista normal e os acertos da busca.
 * (De propósito nada disso vai pro índice — é estado que muda o tempo todo,
 * o tantivy só ordena por relevância e o SQLite filtra.)
 */
function patchHits(
  hits: SearchHit[] | null,
  id: number,
  patch: Partial<ArticleRow>,
): SearchHit[] | null {
  if (!hits) return null;
  return hits.map((h) =>
    h.article.id === id ? { ...h, article: { ...h.article, ...patch } } : h,
  );
}

interface FeedState {
  feeds: FeedRow[];
  filter: ListFilter;
  articles: ArticleRow[];
  /** Texto da busca. */
  query: string;
  /** Período da busca full-text. */
  period: SearchPeriod;
  /** Acertos da busca full-text; `null` = não estamos buscando. */
  hits: SearchHit[] | null;
  searching: boolean;
  /** Backfill do índice em andamento (primeira execução com artigos). */
  indexing: { done: number; total: number } | null;
  listLoading: boolean;
  current: ArticleFull | null;
  refreshing: boolean;

  loadFeeds: () => Promise<void>;
  setFilter: (f: ListFilter) => Promise<void>;
  setQuery: (q: string) => void;
  setPeriod: (p: SearchPeriod) => Promise<void>;
  /** Roda a busca full-text com query/filtro/período atuais. */
  runSearch: () => Promise<void>;
  setIndexing: (v: { done: number; total: number } | null) => void;
  reloadArticles: () => Promise<void>;
  openArticle: (id: number) => Promise<void>;
  addFeed: (url: string) => Promise<boolean>;
  removeFeed: (feedId: number) => Promise<void>;
  moveFeed: (feedId: number, folder: string | null) => Promise<void>;
  refreshAll: () => Promise<void>;
  markAllRead: () => Promise<void>;
  toggleFavorite: () => Promise<void>;
  /** Alterna "ler depois" do artigo aberto. */
  toggleLater: () => Promise<void>;
  markUnread: () => Promise<void>;
  /** Alterna lido/não-lido de um artigo da lista (navegação por teclado: m). */
  toggleReadArticle: (id: number) => Promise<void>;
  /** Alterna favorito de um artigo da lista (navegação por teclado: s). */
  toggleFavoriteArticle: (id: number) => Promise<void>;
  /** Alterna "ler depois" de um artigo da lista (navegação por teclado: l). */
  toggleLaterArticle: (id: number) => Promise<void>;
}

export const useFeed = create<FeedState>((set, get) => ({
  feeds: [],
  filter: { kind: "all" },
  articles: [],
  query: "",
  period: "any",
  hits: null,
  searching: false,
  indexing: null,
  listLoading: false,
  current: null,
  refreshing: false,

  loadFeeds: async () => {
    const feeds = await backend.listFeeds().catch(() => [] as FeedRow[]);
    set({ feeds });
  },

  // Quem dispara a busca é o componente (com debounce) — aqui só o texto.
  setQuery: (query) => {
    set({ query });
    if (!isSearchable(query)) set({ hits: null, searching: false });
  },

  setPeriod: async (period) => {
    set({ period });
    await get().runSearch();
  },

  setIndexing: (indexing) => set({ indexing }),

  runSearch: async () => {
    const { query, filter, period } = get();
    if (!backend.isTauri || !isSearchable(query)) {
      set({ hits: null, searching: false });
      return;
    }
    set({ searching: true });
    try {
      const hits = await backend.searchArticles(query, filter, sinceMsFor(period));
      // A resposta pode chegar depois do usuário já ter mudado o texto;
      // nesse caso ela é lixo e o pedido mais novo manda.
      if (get().query !== query) return;
      set({ hits, searching: false });
    } catch (e) {
      set({ hits: [], searching: false });
      useUi.getState().pushToast("error", t("toast.searchFailed", { error: String(e) }));
    }
  },

  setFilter: async (filter) => {
    set({ filter, current: null });
    await get().reloadArticles();
    // O filtro da barra lateral também é filtro da busca — se havia busca
    // aberta, ela é refeita no novo escopo em vez de mostrar acertos velhos.
    await get().runSearch();
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
      hits: patchHits(s.hits, id, { read: true }),
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
    await get().runSearch(); // o feed removido saiu do índice também
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
    set({
      articles: get().articles.map((x) => (x.id === id ? { ...x, read } : x)),
      hits: patchHits(get().hits, id, { read }),
    });
    void get().loadFeeds();
  },

  toggleFavoriteArticle: async (id) => {
    const favorite = await backend.toggleFavorite(id).catch(() => undefined);
    if (favorite === undefined) return;
    const cur = get().current;
    set({
      articles: get().articles.map((x) => (x.id === id ? { ...x, favorite } : x)),
      hits: patchHits(get().hits, id, { favorite }),
      current: cur && cur.id === id ? { ...cur, favorite } : cur,
    });
  },

  toggleLaterArticle: async (id) => {
    const later = await backend.toggleLater(id).catch(() => undefined);
    if (later === undefined) return;
    const cur = get().current;
    // O item NÃO some da visão "ler depois" ao ser desmarcado: sumir debaixo
    // do cursor é pior que ficar lá sem a marca até a próxima carga (mesmo
    // comportamento dos favoritos).
    set({
      articles: get().articles.map((x) => (x.id === id ? { ...x, later } : x)),
      hits: patchHits(get().hits, id, { later }),
      current: cur && cur.id === id ? { ...cur, later } : cur,
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
      await get().runSearch(); // artigos novos podem bater na busca aberta
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
    await get().runSearch();
  },

  toggleFavorite: async () => {
    const cur = get().current;
    if (!cur) return;
    const favorite = await backend.toggleFavorite(cur.id).catch(() => cur.favorite);
    set({
      current: { ...cur, favorite },
      articles: get().articles.map((a) => (a.id === cur.id ? { ...a, favorite } : a)),
      hits: patchHits(get().hits, cur.id, { favorite }),
    });
  },

  toggleLater: async () => {
    const cur = get().current;
    if (!cur) return;
    const later = await backend.toggleLater(cur.id).catch(() => cur.later);
    set({
      current: { ...cur, later },
      articles: get().articles.map((a) => (a.id === cur.id ? { ...a, later } : a)),
      hits: patchHits(get().hits, cur.id, { later }),
    });
  },

  markUnread: async () => {
    const cur = get().current;
    if (!cur) return;
    await backend.markRead(cur.id, false).catch(() => {});
    set({
      current: null,
      articles: get().articles.map((a) => (a.id === cur.id ? { ...a, read: false } : a)),
      hits: patchHits(get().hits, cur.id, { read: false }),
    });
    void get().loadFeeds();
  },
}));
