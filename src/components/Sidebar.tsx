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
  const { setFilter, addFeed, refreshAll, removeFeed, moveFeed } = useFeed.getState();
  const setSettingsOpen = useUi((s) => s.setSettingsOpen);
  const pushToast = useUi((s) => s.pushToast);

  const [url, setUrl] = useState("");
  const [adding, setAdding] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState<{ id: number; title: string } | null>(null);
  const [moveTarget, setMoveTarget] = useState<{ id: number; title: string; folder: string } | null>(null);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const totalUnread = feeds.reduce((acc, f) => acc + f.unread, 0);

  // Agrupa: feeds sem pasta primeiro (soltos), depois cada pasta em ordem.
  const loose = feeds.filter((f) => !f.folder);
  const folderNames = [...new Set(feeds.map((f) => f.folder).filter((x): x is string => !!x))].sort(
    (a, b) => a.localeCompare(b),
  );
  const toggleFolder = (name: string) =>
    setCollapsed((s) => {
      const n = new Set(s);
      n.has(name) ? n.delete(name) : n.add(name);
      return n;
    });

  const feedRow = (f: (typeof feeds)[number]) => (
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
        className="feed-move"
        title={t("feed.move")}
        onClick={() => setMoveTarget({ id: f.id, title: f.title, folder: f.folder ?? "" })}
      >
        🗂
      </button>
      <button
        className="feed-remove"
        title={t("feed.remove")}
        onClick={() => setConfirmRemove({ id: f.id, title: f.title })}
      >
        ×
      </button>
    </div>
  );

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
        {loose.map(feedRow)}
        {folderNames.map((name) => {
          const group = feeds.filter((f) => f.folder === name);
          const unread = group.reduce((a, f) => a + f.unread, 0);
          const isCollapsed = collapsed.has(name);
          return (
            <div key={name} className="feed-folder">
              <button className="folder-head" onClick={() => toggleFolder(name)}>
                <span className="folder-caret">{isCollapsed ? "▸" : "▾"}</span>
                <span className="side-name">{name}</span>
                {unread > 0 && <span className="badge">{unread}</span>}
              </button>
              {!isCollapsed && group.map(feedRow)}
            </div>
          );
        })}
      </div>

      <div className="side-foot">
        <button onClick={() => void doImport()}>{t("side.import")}</button>
        <button onClick={() => void doExport()}>{t("side.export")}</button>
        <button title={t("side.settingsTitle")} onClick={() => setSettingsOpen(true)}>
          ⚙
        </button>
      </div>

      {moveTarget && (
        <div className="modal-backdrop" onClick={() => setMoveTarget(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>{t("feed.moveTitle", { title: moveTarget.title })}</h2>
            <label className="field">
              <span>{t("feed.folderLabel")}</span>
              <input
                autoFocus
                list="folder-options"
                value={moveTarget.folder}
                placeholder={t("feed.folderPlaceholder")}
                spellCheck={false}
                onChange={(e) => setMoveTarget({ ...moveTarget, folder: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    void moveFeed(moveTarget.id, moveTarget.folder.trim() || null);
                    setMoveTarget(null);
                  }
                }}
              />
            </label>
            <datalist id="folder-options">
              {folderNames.map((n) => (
                <option key={n} value={n} />
              ))}
            </datalist>
            <div className="modal-actions">
              <button onClick={() => setMoveTarget(null)}>{t("dlg.cancel")}</button>
              <button
                className="primary"
                onClick={() => {
                  void moveFeed(moveTarget.id, moveTarget.folder.trim() || null);
                  setMoveTarget(null);
                }}
              >
                {t("dlg.ok")}
              </button>
            </div>
          </div>
        </div>
      )}

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
