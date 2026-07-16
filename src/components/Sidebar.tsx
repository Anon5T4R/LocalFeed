import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as backend from "../lib/backend";
import { t } from "../lib/i18n";
import { useFeed } from "../state/store";
import { useUi } from "../state/ui";

/** Sidebar: assinar, visões (todos/não lidos/favoritos), feeds, OPML. */
export default function Sidebar() {
  const feeds = useFeed((s) => s.feeds);
  const filter = useFeed((s) => s.filter);
  const refreshing = useFeed((s) => s.refreshing);
  const { setFilter, addFeed, refreshAll, removeFeed } = useFeed.getState();
  const setSettingsOpen = useUi((s) => s.setSettingsOpen);
  const pushToast = useUi((s) => s.pushToast);

  const [url, setUrl] = useState("");
  const [adding, setAdding] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState<{ id: number; title: string } | null>(null);

  const totalUnread = feeds.reduce((acc, f) => acc + f.unread, 0);

  const submit = async () => {
    const u = url.trim();
    if (!u || adding) return;
    setAdding(true);
    const ok = await addFeed(u);
    setAdding(false);
    if (ok) setUrl("");
  };

  const doImport = async () => {
    const picked = await open({ filters: [{ name: "OPML", extensions: ["opml", "xml"] }] });
    if (typeof picked !== "string") return;
    try {
      const r = await backend.importOpml(picked);
      pushToast("ok", t("toast.imported", { added: r.added, skipped: r.skipped }));
      await useFeed.getState().loadFeeds();
    } catch (e) {
      pushToast("error", t("toast.importFailed", { error: String(e) }));
    }
  };

  const doExport = async () => {
    const dest = await save({ defaultPath: "localfeed.opml", filters: [{ name: "OPML", extensions: ["opml"] }] });
    if (typeof dest !== "string") return;
    try {
      await backend.exportOpml(dest);
      pushToast("ok", t("toast.exported"));
    } catch (e) {
      pushToast("error", String(e));
    }
  };

  const viewBtn = (
    kind: "all" | "unread" | "favorites",
    label: string,
    badge?: number,
  ) => (
    <button
      className={`side-item ${filter.kind === kind ? "active" : ""}`}
      onClick={() => void setFilter({ kind })}
    >
      <span className="side-name">{label}</span>
      {badge !== undefined && badge > 0 && <span className="badge">{badge}</span>}
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="add-row">
        <input
          value={url}
          placeholder={t("side.addPlaceholder")}
          spellCheck={false}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void submit();
          }}
        />
        <button className="primary" disabled={!url.trim() || adding} onClick={() => void submit()}>
          {adding ? "…" : t("side.add")}
        </button>
      </div>

      <div className="side-views">
        {viewBtn("all", t("side.all"))}
        {viewBtn("unread", t("side.unread"), totalUnread)}
        {viewBtn("favorites", t("side.favorites"))}
      </div>

      <div className="side-title-row">
        <span className="side-title">{t("side.feeds")}</span>
        <button
          className={`icon-btn ${refreshing ? "spin" : ""}`}
          title={t("side.refresh")}
          disabled={refreshing}
          onClick={() => void refreshAll()}
        >
          ⟳
        </button>
      </div>

      <div className="feed-list">
        {feeds.length === 0 && <div className="muted small side-empty">{t("side.empty")}</div>}
        {feeds.map((f) => (
          <div key={f.id} className="feed-row-wrap">
            <button
              className={`side-item ${filter.kind === "feed" && filter.feedId === f.id ? "active" : ""} ${f.lastError ? "has-error" : ""}`}
              title={f.lastError ? t("feed.errorTitle", { error: f.lastError }) : f.url}
              onClick={() => void setFilter({ kind: "feed", feedId: f.id })}
            >
              <span className="side-name">{f.title}</span>
              {f.unread > 0 && <span className="badge">{f.unread}</span>}
            </button>
            <button
              className="feed-remove"
              title={t("feed.remove")}
              onClick={() => setConfirmRemove({ id: f.id, title: f.title })}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      <div className="side-foot">
        <button onClick={() => void doImport()}>{t("side.import")}</button>
        <button onClick={() => void doExport()}>{t("side.export")}</button>
        <button title={t("side.settingsTitle")} onClick={() => setSettingsOpen(true)}>
          ⚙
        </button>
      </div>

      {confirmRemove && (
        <div className="modal-backdrop" onClick={() => setConfirmRemove(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <p>{t("feed.removeConfirm", { title: confirmRemove.title })}</p>
            <div className="modal-actions">
              <button onClick={() => setConfirmRemove(null)}>{t("dlg.cancel")}</button>
              <button
                className="danger"
                onClick={() => {
                  void removeFeed(confirmRemove.id);
                  setConfirmRemove(null);
                }}
              >
                {t("dlg.remove")}
              </button>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
}
