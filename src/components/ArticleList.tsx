import { useEffect, useRef, useState } from "react";
import { isTauri } from "../lib/backend";
import { formatWhen } from "../lib/sanitize";
import { SEARCH_PERIODS, isSearchable } from "../lib/search";
import { localeTag, t } from "../lib/i18n";
import type { ArticleRow, SearchPeriod } from "../lib/types";
import { useFeed } from "../state/store";

/** Espera o usuário parar de digitar antes de ir ao índice. */
const DEBOUNCE_MS = 200;

const PERIOD_LABEL = {
  any: "search.any",
  week: "search.week",
  month: "search.month",
  year: "search.year",
} as const;

/** Lista de artigos do filtro atual (não lido = negrito) ou acertos da busca. */
export default function ArticleList() {
  const articles = useFeed((s) => s.articles);
  const query = useFeed((s) => s.query);
  const period = useFeed((s) => s.period);
  const hits = useFeed((s) => s.hits);
  const searching = useFeed((s) => s.searching);
  const indexing = useFeed((s) => s.indexing);
  const loading = useFeed((s) => s.listLoading);
  const current = useFeed((s) => s.current);
  const {
    openArticle,
    markAllRead,
    setQuery,
    setPeriod,
    runSearch,
    toggleReadArticle,
    toggleFavoriteArticle,
    toggleLaterArticle,
  } = useFeed.getState();
  const [sel, setSel] = useState(0);
  const bodyRef = useRef<HTMLDivElement>(null);

  const labels = { now: t("time.now"), min: t("time.min"), hour: t("time.hour") };

  // Digitou → busca full-text no backend (debounce). Fora do Tauri (smoke em
  // navegador) não há índice: cai no filtro por substring da lista carregada.
  useEffect(() => {
    if (!isTauri || !isSearchable(query)) return;
    const id = setTimeout(() => void runSearch(), DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [query, runSearch]);

  const q = query.trim().toLowerCase();
  const searchMode = hits !== null;
  const localFallback =
    !isTauri && q
      ? articles.filter(
          (a) =>
            a.title.toLowerCase().includes(q) ||
            a.excerpt.toLowerCase().includes(q) ||
            a.feedTitle.toLowerCase().includes(q),
        )
      : articles;

  // Uma lista só, venha ela da busca ou do filtro — o resto do componente
  // (teclado, seleção, rolagem) não precisa saber de onde veio.
  const shown: { article: ArticleRow; snippet: string | null }[] = searchMode
    ? hits.map((h) => ({ article: h.article, snippet: h.snippet }))
    : localFallback.map((a) => ({ article: a, snippet: null }));

  // Mantém a seleção dentro dos limites quando a lista muda.
  const selClamped = Math.min(sel, Math.max(0, shown.length - 1));

  // Navegação por teclado (j/k mover, o/Enter abrir, m lido, s favoritar).
  // Ignora quando o foco está num campo de texto (busca) ou no select.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement;
      if (
        el &&
        (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT")
      ) {
        return;
      }
      if (shown.length === 0) return;
      const cur = Math.min(sel, shown.length - 1);
      if (e.key === "j" || e.key === "ArrowDown") {
        e.preventDefault();
        setSel(Math.min(shown.length - 1, cur + 1));
      } else if (e.key === "k" || e.key === "ArrowUp") {
        e.preventDefault();
        setSel(Math.max(0, cur - 1));
      } else if (e.key === "o" || e.key === "Enter") {
        e.preventDefault();
        void openArticle(shown[cur].article.id);
      } else if (e.key === "m") {
        e.preventDefault();
        void toggleReadArticle(shown[cur].article.id);
      } else if (e.key === "s") {
        e.preventDefault();
        void toggleFavoriteArticle(shown[cur].article.id);
      } else if (e.key === "l") {
        e.preventDefault();
        void toggleLaterArticle(shown[cur].article.id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [shown, sel, openArticle, toggleReadArticle, toggleFavoriteArticle, toggleLaterArticle]);

  // Rola a linha selecionada pra dentro da vista.
  useEffect(() => {
    const body = bodyRef.current;
    const row = body?.querySelector<HTMLElement>(`[data-idx="${selClamped}"]`);
    row?.scrollIntoView({ block: "nearest" });
  }, [selClamped]);

  return (
    <div className="article-list">
      <div className="list-head">
        <input
          className="list-search"
          value={query}
          placeholder={t("list.search")}
          title={t("search.hint")}
          spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
        />
        {isTauri && isSearchable(query) ? (
          <select
            className="period-select"
            value={period}
            title={t("search.periodTitle")}
            onChange={(e) => void setPeriod(e.target.value as SearchPeriod)}
          >
            {SEARCH_PERIODS.map((p) => (
              <option key={p} value={p}>
                {t(PERIOD_LABEL[p])}
              </option>
            ))}
          </select>
        ) : (
          <button className="small-btn" onClick={() => void markAllRead()}>
            {t("list.markAll")}
          </button>
        )}
      </div>

      {/* Backfill: quem já tinha artigos ganha o índice na 1ª execução — e
          precisa saber que é isso, não o app travado. */}
      {indexing && (
        <div className="index-bar" title={t("search.indexingHint")}>
          {t("search.indexing", { done: indexing.done, total: indexing.total })}
        </div>
      )}

      <div className="list-body" ref={bodyRef}>
        {loading && <div className="muted list-msg">{t("list.loading")}</div>}
        {searching && shown.length === 0 && (
          <div className="muted list-msg">{t("search.running")}</div>
        )}
        {!loading && !searching && shown.length === 0 && (
          <div className="muted list-msg">{q ? t("list.noResults") : t("list.empty")}</div>
        )}
        {searchMode && shown.length > 0 && (
          <div className="muted list-msg small">{t("search.results", { n: shown.length })}</div>
        )}
        {shown.map(({ article: a, snippet }, i) => (
          <button
            key={a.id}
            data-idx={i}
            className={`article-row ${a.read ? "" : "unread"} ${current?.id === a.id ? "active" : ""} ${i === selClamped ? "selected" : ""}`}
            onClick={() => {
              setSel(i);
              void openArticle(a.id);
            }}
          >
            <span className="art-title">
              {a.favorite && <span className="fav-star">★ </span>}
              {a.later && <span className="later-dot">◉ </span>}
              {a.title}
            </span>
            <span className="art-meta">
              {a.feedTitle} · {formatWhen(a.publishedMs, localeTag(), labels)}
            </span>
            {snippet ? (
              <span
                className="art-snippet"
                // O tantivy já escapa o texto e só injeta <b> nos termos que
                // bateram — nada de HTML do artigo chega aqui.
                dangerouslySetInnerHTML={{ __html: snippet }}
              />
            ) : (
              a.excerpt && <span className="art-excerpt">{a.excerpt}</span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
