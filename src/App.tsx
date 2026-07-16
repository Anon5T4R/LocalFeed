import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import * as backend from "./lib/backend";
import { t } from "./lib/i18n";
import ArticleList from "./components/ArticleList";
import ReadingPane from "./components/ReadingPane";
import SettingsModal from "./components/SettingsModal";
import Sidebar from "./components/Sidebar";
import Toasts from "./components/Toasts";
import { useFeed } from "./state/store";
import { useUi } from "./state/ui";

export default function App() {
  // Boot: feeds + artigos; .opml passado no launch importa direto.
  useEffect(() => {
    if (!backend.isTauri) return;
    const feed = useFeed.getState();
    void feed.loadFeeds().then(() => feed.reloadArticles());
    void backend.getStartupFile().then(async (f) => {
      if (!f) return;
      try {
        const r = await backend.importOpml(f);
        useUi.getState().pushToast("ok", t("toast.imported", { added: r.added, skipped: r.skipped }));
        await feed.loadFeeds();
      } catch (e) {
        useUi.getState().pushToast("error", t("toast.importFailed", { error: String(e) }));
      }
    });
  }, []);

  // 2ª instância com .opml + progresso do refresh (toast informativo).
  useEffect(() => {
    if (!backend.isTauri) return;
    const un1 = listen<string>("open-opml", async (e) => {
      try {
        const r = await backend.importOpml(e.payload);
        useUi.getState().pushToast("ok", t("toast.imported", { added: r.added, skipped: r.skipped }));
        await useFeed.getState().loadFeeds();
      } catch (err) {
        useUi.getState().pushToast("error", t("toast.importFailed", { error: String(err) }));
      }
    });
    return () => {
      void un1.then((f) => f());
    };
  }, []);

  // F5 = atualizar tudo.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "F5") {
        e.preventDefault();
        void useFeed.getState().refreshAll();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="app">
      <Sidebar />
      <ArticleList />
      <ReadingPane />
      <SettingsModal />
      <Toasts />
    </div>
  );
}
