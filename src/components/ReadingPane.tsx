import { useMemo } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { sanitizeHtml, formatWhen } from "../lib/sanitize";
import { localeTag, t } from "../lib/i18n";
import { useFeed } from "../state/store";
import { useUi } from "../state/ui";

/** Painel de leitura: artigo limpo, links abrem no navegador. */
export default function ReadingPane() {
  const article = useFeed((s) => s.current);
  const { toggleFavorite, toggleLater, markUnread } = useFeed.getState();

  const html = useMemo(
    () => (article?.contentHtml ? sanitizeHtml(article.contentHtml) : null),
    [article?.id, article?.contentHtml], // eslint-disable-line react-hooks/exhaustive-deps
  );

  if (!article) {
    return <div className="reading empty muted">{t("read.select")}</div>;
  }

  const openExternal = () => {
    if (article.url) {
      void openUrl(article.url).catch((e) =>
        useUi.getState().pushToast("error", t("toast.openFailed", { error: String(e) })),
      );
    }
  };

  const labels = { now: t("time.now"), min: t("time.min"), hour: t("time.hour") };

  return (
    <div className="reading">
      <div className="reading-toolbar">
        <button
          className={article.favorite ? "active" : ""}
          title={article.favorite ? t("read.unfavorite") : t("read.favorite")}
          onClick={() => void toggleFavorite()}
        >
          {article.favorite ? "★" : "☆"}
        </button>
        <button
          className={article.later ? "active" : ""}
          title={article.later ? t("read.unlater") : t("read.later")}
          onClick={() => void toggleLater()}
        >
          {article.later ? "◉" : "◎"}
        </button>
        <button title={t("read.markUnread")} onClick={() => void markUnread()}>
          ◌
        </button>
        <span className="toolbar-fill" />
        {article.url && (
          <button onClick={openExternal}>{t("read.openBrowser")} ↗</button>
        )}
      </div>

      <div className="reading-scroll">
        <h1 className="reading-title">{article.title}</h1>
        <div className="reading-meta muted">
          {article.feedTitle}
          {article.author ? ` · ${article.author}` : ""}
          {article.publishedMs ? ` · ${formatWhen(article.publishedMs, localeTag(), labels)}` : ""}
        </div>
        {html ? (
          <div
            className="reading-content"
            // Sanitizado em sanitizeHtml (scripts/on*/javascript: removidos).
            dangerouslySetInnerHTML={{ __html: html }}
            onClick={(e) => {
              // Todo link abre no navegador do sistema, nunca dentro do app.
              const a = (e.target as HTMLElement).closest("a");
              if (a?.href) {
                e.preventDefault();
                void openUrl(a.href).catch(() => {});
              }
            }}
          />
        ) : (
          <p className="muted">{t("read.noContent")}</p>
        )}
      </div>
    </div>
  );
}
