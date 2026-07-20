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
  "side.later": "Ler depois",
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
  "list.noResults": "Nada encontrado.",
  "list.search": "Buscar artigos…",
  "list.markAll": "Marcar tudo como lido",
  "list.loading": "Carregando…",

  // Leitura
  "read.select": "Escolha um artigo pra ler.",
  "read.openBrowser": "Abrir no navegador",
  "read.favorite": "Favoritar",
  "read.unfavorite": "Tirar dos favoritos",
  "read.later": "Marcar pra ler depois",
  "read.unlater": "Tirar de ler depois",
  "read.markUnread": "Marcar como não lido",
  "read.noContent": "Sem conteúdo — abra no navegador.",
  "read.offlineNote": "Não deu pra baixar o artigo completo (offline?). Mostrando o resumo do feed.",

  // Feed (menu)
  "feed.remove": "Remover feed",
  "feed.move": "Mover pra pasta",
  "feed.moveTitle": "Mover “{title}” pra pasta",
  "feed.folderLabel": "Pasta",
  "feed.folderPlaceholder": "Nome da pasta (vazio = sem pasta)",
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

  // Busca full-text (índice tantivy — ver src-tauri/src/search.rs)
  "search.hint": "Busca no texto completo dos artigos baixados (título, conteúdo, autor e feed)",
  "search.running": "Buscando…",
  "search.results": "{n} resultados",
  "search.periodTitle": "Período",
  "search.any": "Qualquer data",
  "search.week": "Última semana",
  "search.month": "Último mês",
  "search.year": "Último ano",
  "search.indexing": "Indexando artigos… {done}/{total}",
  "search.indexingHint": "Primeira execução: o índice de busca está sendo criado a partir dos artigos que você já tem. Dá pra ler normalmente enquanto isso.",
  "search.indexSize": "Índice de busca",
  "settings.imageCache": "Imagens em cache",
  "settings.imageCacheHint": "As imagens dos artigos são baixadas uma vez e exibidas do disco — assim abrir um artigo não avisa o site que você o leu.",
  "search.indexHint": "O índice é derivado dos artigos — se for apagado, o app o reconstrói sozinho na próxima abertura.",
  "toast.searchFailed": "Falha na busca: {error}",

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
  "settings.themeNature": "Natureza",
  "settings.themeDarkBlue": "Azul escuro",
  "settings.themeCalmGreen": "Verde calmo",
  "settings.themePastelPink": "Rosa pastel",
  "settings.themePunkPrincess": "PunkPrincess",
  "settings.language": "Idioma",
  "settings.autoRefresh": "Atualização automática",
  "settings.off": "Desligada",
  "settings.min": "{n} min",
  "settings.storage": "Dados e armazenamento",
  "settings.storagePath": "Pasta de dados",
  "settings.storageOpen": "Abrir pasta",
  "settings.storageSize": "Tamanho do banco",
  "settings.storageCounts": "{n} artigos ({cached} com leitura em cache, {favs} favoritos, {later} pra ler depois)",
  "settings.clearCache": "Limpar cache de leitura",
  "settings.clearCacheHint": "Remove o texto completo e as imagens baixadas dos artigos — lidos e favoritos ficam; tudo volta a ser baixado ao abrir.",
  "settings.clearCacheConfirm": "Limpar o conteúdo em cache de {n} artigos? Os artigos e o estado de lido/favorito ficam.",
  "settings.deleteOld": "Apagar artigos antigos",
  "settings.deleteOldHint": "Apaga artigos antigos que não sejam favoritos nem estejam marcados pra ler depois — esses dois nunca são apagados.",
  "settings.deleteOldConfirm": "Apagar artigos com mais de {days} dias? Favoritos e marcados pra ler depois nunca são apagados.",
  "settings.days": "{n} dias",
  "toast.cacheCleared": "Cache de leitura limpo ({n} artigos)",
  "toast.oldDeleted": "{n} artigos antigos apagados",
  "toast.nothingToDelete": "Nada pra apagar",
  "toast.storageFailed": "Falha: {error}",
  "dlg.clear": "Limpar",
  "dlg.delete": "Apagar",
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
  "side.later": "Read later",
  "side.feeds": "Feeds",
  "side.addPlaceholder": "Site or feed URL…",
  "side.add": "Subscribe",
  "side.refresh": "Refresh all",
  "side.import": "Import OPML…",
  "side.export": "Export OPML…",
  "side.settingsTitle": "Settings",
  "side.empty": "No feeds yet — subscribe to a site above or import an OPML.",

  "list.empty": "Nothing here.",
  "list.noResults": "Nothing found.",
  "list.search": "Search articles…",
  "list.markAll": "Mark all as read",
  "list.loading": "Loading…",

  "read.select": "Pick an article to read.",
  "read.openBrowser": "Open in browser",
  "read.favorite": "Favorite",
  "read.unfavorite": "Remove favorite",
  "read.later": "Mark to read later",
  "read.unlater": "Remove from read later",
  "read.markUnread": "Mark as unread",
  "read.noContent": "No content — open in the browser.",
  "read.offlineNote": "Couldn't fetch the full article (offline?). Showing the feed summary.",

  "feed.remove": "Remove feed",
  "feed.move": "Move to folder",
  "feed.moveTitle": "Move “{title}” to folder",
  "feed.folderLabel": "Folder",
  "feed.folderPlaceholder": "Folder name (empty = no folder)",
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
  "settings.themeNature": "Nature",
  "settings.themeDarkBlue": "Dark blue",
  "settings.themeCalmGreen": "Calm green",
  "settings.themePastelPink": "Pastel pink",
  "settings.themePunkPrincess": "PunkPrincess",
  "settings.language": "Language",
  "settings.autoRefresh": "Auto-refresh",
  "settings.off": "Off",
  "settings.min": "{n} min",
  "settings.storage": "Data & storage",
  "settings.storagePath": "Data folder",
  "settings.storageOpen": "Open folder",
  "settings.storageSize": "Database size",
  "settings.storageCounts": "{n} articles ({cached} with cached reading, {favs} favorites, {later} to read later)",
  "settings.clearCache": "Clear reading cache",
  "settings.clearCacheHint": "Removes the downloaded full text and images of articles — read state and favorites stay; everything is fetched again on open.",
  "settings.clearCacheConfirm": "Clear the cached content of {n} articles? The articles and their read/favorite state stay.",
  "settings.deleteOld": "Delete old articles",
  "settings.deleteOldHint": "Deletes old articles that are neither favorites nor marked to read later — those two are never deleted.",
  "settings.deleteOldConfirm": "Delete articles older than {days} days? Favorites and articles marked to read later are never deleted.",
  "settings.days": "{n} days",
  "toast.cacheCleared": "Reading cache cleared ({n} articles)",
  "toast.oldDeleted": "{n} old articles deleted",
  "toast.nothingToDelete": "Nothing to delete",
  "toast.storageFailed": "Failed: {error}",
  "dlg.clear": "Clear",
  "dlg.delete": "Delete",
  "search.hint": "Searches the full text of downloaded articles (title, content, author and feed)",
  "search.running": "Searching…",
  "search.results": "{n} results",
  "search.periodTitle": "Period",
  "search.any": "Any date",
  "search.week": "Last week",
  "search.month": "Last month",
  "search.year": "Last year",
  "search.indexing": "Indexing articles… {done}/{total}",
  "search.indexingHint": "First run: the search index is being built from the articles you already have. You can keep reading in the meantime.",
  "search.indexSize": "Search index",
  "settings.imageCache": "Cached images",
  "settings.imageCacheHint": "Article images are downloaded once and shown from disk — so opening an article does not tell the site you read it.",
  "search.indexHint": "The index is derived from the articles — if deleted, the app rebuilds it on the next launch.",
  "toast.searchFailed": "Search failed: {error}",
  "settings.network":
    "Network: LocalFeed only reaches the feeds you subscribed to (the one \"non-offline\" exception — reading what's already downloaded works without internet). No account, no tracker, no algorithm.",
  "settings.about":
    " — 100% local RSS/Atom reader: subscribe to sites, read the clean article (reader mode), mark read/favorites and carry everything in OPML. Part of the Local suite.",
};

const es: Record<MessageKey, string> = {
  "side.all": "Todos",
  "side.unread": "No leídos",
  "side.favorites": "Favoritos",
  "side.later": "Leer después",
  "side.feeds": "Feeds",
  "side.addPlaceholder": "URL del sitio o del feed…",
  "side.add": "Suscribirse",
  "side.refresh": "Actualizar todo",
  "side.import": "Importar OPML…",
  "side.export": "Exportar OPML…",
  "side.settingsTitle": "Configuración",
  "side.empty": "Ningún feed todavía — suscríbete a un sitio arriba o importa un OPML.",

  "list.empty": "Nada por aquí.",
  "list.noResults": "No se encontró nada.",
  "list.search": "Buscar artículos…",
  "list.markAll": "Marcar todo como leído",
  "list.loading": "Cargando…",

  "read.select": "Elige un artículo para leer.",
  "read.openBrowser": "Abrir en el navegador",
  "read.favorite": "Favorito",
  "read.unfavorite": "Quitar de favoritos",
  "read.later": "Marcar para leer después",
  "read.unlater": "Quitar de leer después",
  "read.markUnread": "Marcar como no leído",
  "read.noContent": "Sin contenido — ábrelo en el navegador.",
  "read.offlineNote": "No se pudo descargar el artículo completo (¿offline?). Mostrando el resumen del feed.",

  "feed.remove": "Eliminar feed",
  "feed.move": "Mover a carpeta",
  "feed.moveTitle": "Mover «{title}» a carpeta",
  "feed.folderLabel": "Carpeta",
  "feed.folderPlaceholder": "Nombre de la carpeta (vacío = sin carpeta)",
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
  "settings.themeNature": "Naturaleza",
  "settings.themeDarkBlue": "Azul oscuro",
  "settings.themeCalmGreen": "Verde tranquilo",
  "settings.themePastelPink": "Rosa pastel",
  "settings.themePunkPrincess": "PunkPrincess",
  "settings.language": "Idioma",
  "settings.autoRefresh": "Actualización automática",
  "settings.off": "Desactivada",
  "settings.min": "{n} min",
  "settings.storage": "Datos y almacenamiento",
  "settings.storagePath": "Carpeta de datos",
  "settings.storageOpen": "Abrir carpeta",
  "settings.storageSize": "Tamaño de la base",
  "settings.storageCounts": "{n} artículos ({cached} con lectura en caché, {favs} favoritos, {later} para leer después)",
  "settings.clearCache": "Limpiar caché de lectura",
  "settings.clearCacheHint": "Elimina el texto completo y las imágenes descargadas de los artículos — leídos y favoritos se conservan; el texto se vuelve a descargar al abrir.",
  "settings.clearCacheConfirm": "¿Limpiar el contenido en caché de {n} artículos? Los artículos y su estado de leído/favorito se conservan.",
  "settings.deleteOld": "Eliminar artículos antiguos",
  "settings.deleteOldHint": "Elimina artículos antiguos que no sean favoritos ni estén marcados para leer después — esos dos nunca se eliminan.",
  "settings.deleteOldConfirm": "¿Eliminar artículos con más de {days} días? Los favoritos y los marcados para leer después nunca se eliminan.",
  "settings.days": "{n} días",
  "toast.cacheCleared": "Caché de lectura limpiada ({n} artículos)",
  "toast.oldDeleted": "{n} artículos antiguos eliminados",
  "toast.nothingToDelete": "Nada que eliminar",
  "toast.storageFailed": "Error: {error}",
  "dlg.clear": "Limpiar",
  "dlg.delete": "Eliminar",
  "search.hint": "Busca en el texto completo de los artículos descargados (título, contenido, autor y feed)",
  "search.running": "Buscando…",
  "search.results": "{n} resultados",
  "search.periodTitle": "Período",
  "search.any": "Cualquier fecha",
  "search.week": "Última semana",
  "search.month": "Último mes",
  "search.year": "Último año",
  "search.indexing": "Indexando artículos… {done}/{total}",
  "search.indexingHint": "Primera ejecución: el índice de búsqueda se está creando a partir de los artículos que ya tienes. Puedes seguir leyendo mientras tanto.",
  "search.indexSize": "Índice de búsqueda",
  "settings.imageCache": "Imágenes en caché",
  "settings.imageCacheHint": "Las imágenes de los artículos se descargan una vez y se muestran desde el disco — así abrir un artículo no le avisa al sitio que lo leíste.",
  "search.indexHint": "El índice deriva de los artículos — si se borra, la app lo reconstruye en el próximo arranque.",
  "toast.searchFailed": "Error en la búsqueda: {error}",
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
