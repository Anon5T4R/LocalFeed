import { LOCALE_LABELS, setLocale, t, useLocale, type Locale } from "../lib/i18n";
import { useUi, type Theme } from "../state/ui";

/** Configurações: tema e idioma (padrão da suíte). */
export default function SettingsModal() {
  const open = useUi((s) => s.settingsOpen);
  const setOpen = useUi((s) => s.setSettingsOpen);
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);
  const autoRefreshMin = useUi((s) => s.autoRefreshMin);
  const setAutoRefreshMin = useUi((s) => s.setAutoRefreshMin);
  const locale = useLocale();

  if (!open) return null;

  const intervals = [0, 15, 30, 60];

  const themes: { value: Theme; label: string }[] = [
    { value: "system", label: t("settings.themeSystem") },
    { value: "light", label: t("settings.themeLight") },
    { value: "dark", label: t("settings.themeDark") },
  ];

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
    </div>
  );
}
