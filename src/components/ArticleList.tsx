import { formatWhen } from "../lib/sanitize";
import { localeTag, t } from "../lib/i18n";
import { useFeed } from "../state/store";

/** Lista de artigos do filtro atual (não lido = negrito). */
export default function ArticleList() {
  const articles = useFeed((s) => s.articles);
  const loading = useFeed((s) => s.listLoading);
  const current = useFeed((s) => s.current);
  const { openArticle, markAllRead } = useFeed.getState();

  const labels = { now: t("time.now"), min: t("time.min"), hour: t("time.hour") };

  return (
    <div className="article-list">
      <div className="list-head">
        <button className="small-btn" onClick={() => void markAllRead()}>
          {t("list.markAll")}
        </button>
      </div>
      <div className="list-body">
        {loading && <div className="muted list-msg">{t("list.loading")}</div>}
        {!loading && articles.length === 0 && (
          <div className="muted list-msg">{t("list.empty")}</div>
        )}
        {articles.map((a) => (
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
