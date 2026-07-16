/**
 * Sanitiza o HTML do artigo antes de renderizar: remove script/iframe/object/
 * embed/form, atributos on* e URLs javascript:. O conteúdo vem do readability
 * ou do resumo do feed — já é "quase limpo", isto é a rede de segurança.
 */
export function sanitizeHtml(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const forbidden = ["script", "iframe", "object", "embed", "form", "link", "meta", "style"];
  for (const tag of forbidden) {
    for (const el of [...doc.querySelectorAll(tag)]) el.remove();
  }
  const walk = (el: Element) => {
    for (const attr of [...el.attributes]) {
      const name = attr.name.toLowerCase();
      const value = attr.value.trim().toLowerCase();
      if (name.startsWith("on")) el.removeAttribute(attr.name);
      else if ((name === "href" || name === "src") && value.startsWith("javascript:")) {
        el.removeAttribute(attr.name);
      }
    }
    for (const child of [...el.children]) walk(child);
  };
  walk(doc.body);
  return doc.body.innerHTML;
}

/** Data relativa/curta pro item da lista (rótulos vêm traduzidos do caller). */
export function formatWhen(
  ms: number | null,
  localeTag: string,
  labels: { now: string; min: string; hour: string },
): string {
  if (!ms) return "";
  const diff = Date.now() - ms;
  const min = Math.floor(diff / 60000);
  if (min < 1) return labels.now;
  if (min < 60) return labels.min.split("{n}").join(String(min));
  const h = Math.floor(min / 60);
  if (h < 24) return labels.hour.split("{n}").join(String(h));
  return new Intl.DateTimeFormat(localeTag, { dateStyle: "short" }).format(new Date(ms));
}
