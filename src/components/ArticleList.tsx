import { formatWhen } from "../lib/sanitize";
import { localeTag, t } from "../lib/i18n";
import { useFeed } from "../state/store";

/** Lista de artigos do filtro atual (não lido = negrito; busca ao vivo). */
export default function ArticleList() {
  const articles = useFeed((s) => s.articles);
  const query = useFeed((s) => s.query);
  const loading = useFeed((s) => s.listLoading);
  const current = useFeed((s) => s.current);
  const { openArticle, markAllRead, setQuery } = useFeed.getState();

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
      <div className="list-body">
        {loading && <div className="muted list-msg">{t("list.loading")}</div>}
        {!loading && shown.length === 0 && (
          <div className="muted list-msg">{q ? t("list.noResults") : t("list.empty")}</div>
        )}
        {shown.map((a) => (
          <button
            key={a.id}
            className={`article-row ${a.read ? "" : "unread"} ${current?.id === a.id ? "active" : ""}`}
            onClick={() => void openArticle(a.id)}
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
