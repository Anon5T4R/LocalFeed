import { useSyncExternalStore } from "react";

/** i18n leve da UI (padrão da suíte, ver docs/planos/padrao-apps.md). */

export type Locale = "pt" | "en" | "es";

export const LOCALE_LABELS: Record<Locale, string> = {
  pt: "Português",
  en: "English",
  es: "Español",
};

export const LOCALE_TAGS: Record<Locale, string> = {
  pt: "pt-BR",
  en: "en-US",
  es: "es",
};

const LOCALE_KEY = "localfeed.locale";

const pt = {
  // Sidebar
  "side.all": "Todos",
  "side.unread": "Não lidos",
  "side.favorites": "Favoritos",
  "side.feeds": "Feeds",
  "side.addPlaceholder": "URL do site ou do feed…",
  "side.add": "Assinar",
  "side.refresh": "Atualizar tudo",
  "side.import": "Importar OPML…",
  "side.export": "Exportar OPML…",
  "side.settingsTitle": "Configurações",
  "side.empty": "Nenhum feed ainda — assine um site acima ou importe um OPML.",

  // Lista
  "list.empty": "Nada por aqui.",
  "list.markAll": "Marcar tudo como lido",
  "list.loading": "Carregando…",

  // Leitura
  "read.select": "Escolha um artigo pra ler.",
  "read.openBrowser": "Abrir no navegador",
  "read.favorite": "Favoritar",
  "read.unfavorite": "Tirar dos favoritos",
  "read.markUnread": "Marcar como não lido",
  "read.noContent": "Sem conteúdo — abra no navegador.",
  "read.offlineNote": "Não deu pra baixar o artigo completo (offline?). Mostrando o resumo do feed.",

  // Feed (menu)
  "feed.remove": "Remover feed",
  "feed.removeConfirm": "Remover “{title}” e todos os artigos?",
  "feed.errorTitle": "Última atualização falhou: {error}",

  // Refresh / toasts
  "toast.refreshing": "Atualizando {feed}…",
  "toast.refreshDone": "{n} artigos novos",
  "toast.refreshNone": "Nenhum artigo novo",
  "toast.refreshErrors": "{n} feeds falharam (ex.: {first})",
  "toast.added": "Assinado: {title}",
  "toast.addFailed": "Não consegui assinar: {error}",
  "toast.removed": "Feed removido",
  "toast.imported": "{added} feeds importados ({skipped} já existiam) — atualize pra baixar os artigos",
  "toast.importFailed": "Falha no import: {error}",
  "toast.exported": "OPML exportado",
  "toast.openFailed": "Não consegui abrir: {error}",

  // Tempo relativo
  "time.now": "agora",
  "time.min": "{n} min",
  "time.hour": "{n} h",

  // Diálogos
  "dlg.cancel": "Cancelar",
  "dlg.remove": "Remover",
  "dlg.ok": "OK",

  // Settings
  "settings.title": "Configurações",
  "settings.theme": "Tema",
  "settings.themeSystem": "Sistema",
  "settings.themeLight": "Claro",
  "settings.themeDark": "Escuro",
  "settings.language": "Idioma",
  "settings.network":
    "Rede: o LocalFeed só acessa os feeds que você assinou (a única exceção \"não-offline\" — a leitura do que já baixou funciona sem internet). Sem conta, sem tracker, sem algoritmo.",
  "settings.about":
    " — leitor de RSS/Atom 100% local: assine sites, leia o artigo limpo (modo leitura), marque lidos/favoritos e leve tudo em OPML. Parte da suíte Local.",
} as const;

export type MessageKey = keyof typeof pt;

const en: Record<MessageKey, string> = {
  "side.all": "All",
  "side.unread": "Unread",
  "side.favorites": "Favorites",
  "side.feeds": "Feeds",
  "side.addPlaceholder": "Site or feed URL…",
  "side.add": "Subscribe",
  "side.refresh": "Refresh all",
  "side.import": "Import OPML…",
  "side.export": "Export OPML…",
  "side.settingsTitle": "Settings",
  "side.empty": "No feeds yet — subscribe to a site above or import an OPML.",

  "list.empty": "Nothing here.",
  "list.markAll": "Mark all as read",
  "list.loading": "Loading…",

  "read.select": "Pick an article to read.",
  "read.openBrowser": "Open in browser",
  "read.favorite": "Favorite",
  "read.unfavorite": "Remove favorite",
  "read.markUnread": "Mark as unread",
  "read.noContent": "No content — open in the browser.",
  "read.offlineNote": "Couldn't fetch the full article (offline?). Showing the feed summary.",

  "feed.remove": "Remove feed",
  "feed.removeConfirm": "Remove “{title}” and all its articles?",
  "feed.errorTitle": "Last refresh failed: {error}",

  "toast.refreshing": "Refreshing {feed}…",
  "toast.refreshDone": "{n} new articles",
  "toast.refreshNone": "No new articles",
  "toast.refreshErrors": "{n} feeds failed (e.g.: {first})",
  "toast.added": "Subscribed: {title}",
  "toast.addFailed": "Couldn't subscribe: {error}",
  "toast.removed": "Feed removed",
  "toast.imported": "{added} feeds imported ({skipped} already existed) — refresh to fetch articles",
  "toast.importFailed": "Import failed: {error}",
  "toast.exported": "OPML exported",
  "toast.openFailed": "Couldn't open: {error}",

  "time.now": "now",
  "time.min": "{n} min",
  "time.hour": "{n} h",

  "dlg.cancel": "Cancel",
  "dlg.remove": "Remove",
  "dlg.ok": "OK",

  "settings.title": "Settings",
  "settings.theme": "Theme",
  "settings.themeSystem": "System",
  "settings.themeLight": "Light",
  "settings.themeDark": "Dark",
  "settings.language": "Language",
  "settings.network":
    "Network: LocalFeed only reaches the feeds you subscribed to (the one \"non-offline\" exception — reading what's already downloaded works without internet). No account, no tracker, no algorithm.",
  "settings.about":
    " — 100% local RSS/Atom reader: subscribe to sites, read the clean article (reader mode), mark read/favorites and carry everything in OPML. Part of the Local suite.",
};

const es: Record<MessageKey, string> = {
  "side.all": "Todos",
  "side.unread": "No leídos",
  "side.favorites": "Favoritos",
  "side.feeds": "Feeds",
  "side.addPlaceholder": "URL del sitio o del feed…",
  "side.add": "Suscribirse",
  "side.refresh": "Actualizar todo",
  "side.import": "Importar OPML…",
  "side.export": "Exportar OPML…",
  "side.settingsTitle": "Configuración",
  "side.empty": "Ningún feed todavía — suscríbete a un sitio arriba o importa un OPML.",

  "list.empty": "Nada por aquí.",
  "list.markAll": "Marcar todo como leído",
  "list.loading": "Cargando…",

  "read.select": "Elige un artículo para leer.",
  "read.openBrowser": "Abrir en el navegador",
  "read.favorite": "Favorito",
  "read.unfavorite": "Quitar de favoritos",
  "read.markUnread": "Marcar como no leído",
  "read.noContent": "Sin contenido — ábrelo en el navegador.",
  "read.offlineNote": "No se pudo descargar el artículo completo (¿offline?). Mostrando el resumen del feed.",

  "feed.remove": "Eliminar feed",
  "feed.removeConfirm": "¿Eliminar “{title}” y todos sus artículos?",
  "feed.errorTitle": "La última actualización falló: {error}",

  "toast.refreshing": "Actualizando {feed}…",
  "toast.refreshDone": "{n} artículos nuevos",
  "toast.refreshNone": "Ningún artículo nuevo",
  "toast.refreshErrors": "{n} feeds fallaron (p. ej.: {first})",
  "toast.added": "Suscrito: {title}",
  "toast.addFailed": "No se pudo suscribir: {error}",
  "toast.removed": "Feed eliminado",
  "toast.imported": "{added} feeds importados ({skipped} ya existían) — actualiza para bajar los artículos",
  "toast.importFailed": "Error al importar: {error}",
  "toast.exported": "OPML exportado",
  "toast.openFailed": "No se pudo abrir: {error}",

  "time.now": "ahora",
  "time.min": "{n} min",
  "time.hour": "{n} h",

  "dlg.cancel": "Cancelar",
  "dlg.remove": "Eliminar",
  "dlg.ok": "OK",

  "settings.title": "Configuración",
  "settings.theme": "Tema",
  "settings.themeSystem": "Sistema",
  "settings.themeLight": "Claro",
  "settings.themeDark": "Oscuro",
  "settings.language": "Idioma",
  "settings.network":
    "Red: LocalFeed solo accede a los feeds que suscribiste (la única excepción \"no-offline\" — leer lo ya descargado funciona sin internet). Sin cuenta, sin tracker, sin algoritmo.",
  "settings.about":
    " — lector RSS/Atom 100% local: suscríbete a sitios, lee el artículo limpio (modo lectura), marca leídos/favoritos y llévate todo en OPML. Parte de la suite Local.",
};

const DICTS: Record<Locale, Record<MessageKey, string>> = { pt, en, es };

export function detectLocale(): Locale {
  const l = (typeof navigator !== "undefined" ? navigator.language : "pt").toLowerCase();
  if (l.startsWith("en")) return "en";
  if (l.startsWith("es")) return "es";
  return "pt";
}

function loadLocale(): Locale {
  const v = typeof localStorage !== "undefined" ? localStorage.getItem(LOCALE_KEY) : null;
  return v === "pt" || v === "en" || v === "es" ? v : detectLocale();
}

let current: Locale = loadLocale();
const listeners = new Set<() => void>();

export function getLocale(): Locale {
  return current;
}

export function localeTag(): string {
  return LOCALE_TAGS[current];
}

export function setLocale(locale: Locale) {
  if (locale === current) return;
  current = locale;
  try {
    localStorage.setItem(LOCALE_KEY, locale);
  } catch {
    /* localStorage indisponível */
  }
  for (const l of listeners) l();
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function useLocale(): Locale {
  return useSyncExternalStore(subscribe, getLocale);
}

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  let msg: string = DICTS[current][key] ?? pt[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      msg = msg.split(`{${k}}`).join(String(v));
    }
  }
  return msg;
}
