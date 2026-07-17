import { useEffect, useRef, useState } from "react";
import { formatWhen } from "../lib/sanitize";
import { localeTag, t } from "../lib/i18n";
import { useFeed } from "../state/store";

/** Lista de artigos do filtro atual (não lido = negrito; busca ao vivo). */
export default function ArticleList() {
  const articles = useFeed((s) => s.articles);
  const query = useFeed((s) => s.query);
  const loading = useFeed((s) => s.listLoading);
  const current = useFeed((s) => s.current);
  const { openArticle, markAllRead, setQuery, toggleReadArticle, toggleFavoriteArticle } =
    useFeed.getState();
  const [sel, setSel] = useState(0);
  const bodyRef = useRef<HTMLDivElement>(null);

  const labels = { now: t("time.now"), min: t("time.min"), hour: t("time.hour") };

  const q = query.trim().toLowerCase();
  const shown = q
    ? articles.filter(
        (a) =>
          a.title.toLowerCase().includes(q) ||
          a.excerpt.toLowerCase().includes(q) ||
          a.feedTitle.toLowerCase().includes(q),
      )
    : articles;

  // Mantém a seleção dentro dos limites quando a lista muda.
  const selClamped = Math.min(sel, Math.max(0, shown.length - 1));

  // Navegação por teclado (j/k mover, o/Enter abrir, m lido, s favoritar).
  // Ignora quando o foco está num campo de texto (busca).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA")) return;
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
        void openArticle(shown[cur].id);
      } else if (e.key === "m") {
        e.preventDefault();
        void toggleReadArticle(shown[cur].id);
      } else if (e.key === "s") {
        e.preventDefault();
        void toggleFavoriteArticle(shown[cur].id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [shown, sel, openArticle, toggleReadArticle, toggleFavoriteArticle]);

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
          spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button className="small-btn" onClick={() => void markAllRead()}>
          {t("list.markAll")}
        </button>
      </div>
      <div className="list-body" ref={bodyRef}>
        {loading && <div className="muted list-msg">{t("list.loading")}</div>}
        {!loading && shown.length === 0 && (
          <div className="muted list-msg">{q ? t("list.noResults") : t("list.empty")}</div>
        )}
        {shown.map((a, i) => (
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
              {a.title}
            </span>
            <span className="art-meta">
              {a.feedTitle} · {formatWhen(a.publishedMs, localeTag(), labels)}
            </span>
            {a.excerpt && <span className="art-excerpt">{a.excerpt}</span>}
          </button>
        ))}
      </div>
    </div>
  );
}
