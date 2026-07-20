import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import * as backend from "../lib/backend";
import { LOCALE_LABELS, localeTag, setLocale, t, useLocale, type Locale } from "../lib/i18n";
import type { StorageInfo } from "../lib/types";
import { useFeed } from "../state/store";
import { useUi, type Theme } from "../state/ui";

/** Bytes legíveis (B/KB/MB) no locale corrente. */
function fmtBytes(n: number): string {
  const fmt = (v: number) => v.toLocaleString(localeTag(), { maximumFractionDigits: 1 });
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${fmt(n / 1024)} KB`;
  return `${fmt(n / (1024 * 1024))} MB`;
}

type StorageConfirm = { kind: "cache" } | { kind: "old"; days: number };

/** Configurações: tema, idioma, auto-refresh e dados/armazenamento (padrão da suíte). */
export default function SettingsModal() {
  const open = useUi((s) => s.settingsOpen);
  const setOpen = useUi((s) => s.setSettingsOpen);
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);
  const autoRefreshMin = useUi((s) => s.autoRefreshMin);
  const setAutoRefreshMin = useUi((s) => s.setAutoRefreshMin);
  const pushToast = useUi((s) => s.pushToast);
  const locale = useLocale();

  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [days, setDays] = useState(90);
  const [confirm, setConfirm] = useState<StorageConfirm | null>(null);
  const [busy, setBusy] = useState(false);

  const loadInfo = () => backend.storageInfo().then(setInfo).catch(() => setInfo(null));

  useEffect(() => {
    if (open && backend.isTauri) void loadInfo();
  }, [open]);

  if (!open) return null;

  const intervals = [0, 15, 30, 60];
  const oldChoices = [30, 90, 180];

  const themes: { value: Theme; label: string }[] = [
    { value: "system", label: t("settings.themeSystem") },
    { value: "light", label: t("settings.themeLight") },
    { value: "dark", label: t("settings.themeDark") },
    { value: "nature", label: t("settings.themeNature") },
    { value: "darkblue", label: t("settings.themeDarkBlue") },
    { value: "calmgreen", label: t("settings.themeCalmGreen") },
    { value: "pastelpink", label: t("settings.themePastelPink") },
    { value: "punkprincess", label: t("settings.themePunkPrincess") },
  ];

  const doClearCache = async () => {
    setBusy(true);
    try {
      const n = await backend.clearReadabilityCache();
      pushToast("ok", t("toast.cacheCleared", { n }));
    } catch (e) {
      pushToast("error", t("toast.storageFailed", { error: String(e) }));
    } finally {
      setBusy(false);
      setConfirm(null);
      void loadInfo();
    }
  };

  const doDeleteOld = async (d: number) => {
    setBusy(true);
    try {
      const n = await backend.clearOldArticles(d);
      pushToast(n > 0 ? "ok" : "info", n > 0 ? t("toast.oldDeleted", { n }) : t("toast.nothingToDelete"));
      if (n > 0) {
        // A lista e o artigo aberto podem ter sido apagados — recarrega tudo.
        useFeed.setState({ current: null });
        await useFeed.getState().loadFeeds();
        await useFeed.getState().reloadArticles();
      }
    } catch (e) {
      pushToast("error", t("toast.storageFailed", { error: String(e) }));
    } finally {
      setBusy(false);
      setConfirm(null);
      void loadInfo();
    }
  };

  return (
    <div className="modal-backdrop" onClick={() => setOpen(false)}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>{t("settings.title")}</h2>

        <div className="settings-row">
          <span>{t("settings.theme")}</span>
          <div className="segmented">
            {themes.map((th) => (
              <button
                key={th.value}
                className={theme === th.value ? "active" : ""}
                onClick={() => setTheme(th.value)}
              >
                {th.label}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <span>{t("settings.language")}</span>
          <div className="segmented">
            {(Object.keys(LOCALE_LABELS) as Locale[]).map((l) => (
              <button key={l} className={locale === l ? "active" : ""} onClick={() => setLocale(l)}>
                {LOCALE_LABELS[l]}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <span>{t("settings.autoRefresh")}</span>
          <div className="segmented">
            {intervals.map((m) => (
              <button
                key={m}
                className={autoRefreshMin === m ? "active" : ""}
                onClick={() => setAutoRefreshMin(m)}
              >
                {m === 0 ? t("settings.off") : t("settings.min", { n: m })}
              </button>
            ))}
          </div>
        </div>

        {info && (
          <>
            <h3 className="settings-section">{t("settings.storage")}</h3>

            <div className="settings-row storage-path-row">
              <span>{t("settings.storagePath")}</span>
              <div className="storage-path">
                <code title={info.dir}>{info.dir}</code>
                <button onClick={() => void openPath(info.dir).catch((e) => pushToast("error", t("toast.openFailed", { error: String(e) })))}>
                  {t("settings.storageOpen")}
                </button>
              </div>
            </div>

            <div className="settings-row">
              <span>{t("settings.storageSize")}</span>
              <span>
                <strong>{fmtBytes(info.dbBytes)}</strong>
                <span className="muted small storage-counts">
                  {" — "}
                  {t("settings.storageCounts", {
                    n: info.articles,
                    cached: info.cached,
                    favs: info.favorites,
                  })}
                </span>
              </span>
            </div>

            <div className="settings-row">
              <span>
                {t("search.indexSize")}
                <span className="muted small storage-hint">{t("search.indexHint")}</span>
              </span>
              <strong>{fmtBytes(info.indexBytes)}</strong>
            </div>

            <div className="settings-row">
              <span>
                {t("settings.clearCache")}
                <span className="muted small storage-hint">{t("settings.clearCacheHint")}</span>
              </span>
              <button disabled={busy || info.cached === 0} onClick={() => setConfirm({ kind: "cache" })}>
                {t("dlg.clear")}
              </button>
            </div>

            <div className="settings-row">
              <span>
                {t("settings.deleteOld")}
                <span className="muted small storage-hint">{t("settings.deleteOldHint")}</span>
              </span>
              <div className="storage-old">
                <select value={days} onChange={(e) => setDays(Number(e.target.value))}>
                  {oldChoices.map((d) => (
                    <option key={d} value={d}>
                      {t("settings.days", { n: d })}
                    </option>
                  ))}
                </select>
                <button
                  className="danger"
                  disabled={busy || info.articles === 0}
                  onClick={() => setConfirm({ kind: "old", days })}
                >
                  {t("dlg.delete")}
                </button>
              </div>
            </div>
          </>
        )}

        <p className="muted about">
          <strong>LocalFeed</strong>
          {t("settings.about")}
        </p>
        <p className="muted about">{t("settings.network")}</p>

        <div className="modal-actions">
          <button className="primary" onClick={() => setOpen(false)}>
            {t("dlg.ok")}
          </button>
        </div>
      </div>

      {confirm && (
        <div className="modal-backdrop" onClick={() => setConfirm(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <p>
              {confirm.kind === "cache"
                ? t("settings.clearCacheConfirm", { n: info?.cached ?? 0 })
                : t("settings.deleteOldConfirm", { days: confirm.days })}
            </p>
            <div className="modal-actions">
              <button disabled={busy} onClick={() => setConfirm(null)}>
                {t("dlg.cancel")}
              </button>
              {confirm.kind === "cache" ? (
                <button className="primary" disabled={busy} onClick={() => void doClearCache()}>
                  {t("dlg.clear")}
                </button>
              ) : (
                <button className="danger" disabled={busy} onClick={() => void doDeleteOld(confirm.days)}>
                  {t("dlg.delete")}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
