// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { formatWhen, sanitizeHtml } from "../sanitize";

describe("sanitizeHtml", () => {
  it("remove script/iframe e mantém o conteúdo", () => {
    const dirty = `<p>Olá</p><script>alert(1)</script><iframe src="x"></iframe><b>mundo</b>`;
    const clean = sanitizeHtml(dirty);
    expect(clean).toContain("<p>Olá</p>");
    expect(clean).toContain("<b>mundo</b>");
    expect(clean).not.toContain("script");
    expect(clean).not.toContain("iframe");
  });

  it("remove atributos on* e javascript:", () => {
    const dirty = `<a href="javascript:alert(1)" onclick="x()">link</a><img src="a.png" onerror="p()">`;
    const clean = sanitizeHtml(dirty);
    expect(clean).not.toContain("javascript:");
    expect(clean).not.toContain("onclick");
    expect(clean).not.toContain("onerror");
    expect(clean).toContain("<a>link</a>");
    expect(clean).toContain('src="a.png"');
  });

  it("mantém links http normais", () => {
    const clean = sanitizeHtml(`<a href="https://ex.com">ok</a>`);
    expect(clean).toContain('href="https://ex.com"');
  });
});

describe("formatWhen", () => {
  const labels = { now: "agora", min: "{n} min", hour: "{n} h" };

  it("relativo pra tempos recentes", () => {
    expect(formatWhen(Date.now() - 30_000, "pt-BR", labels)).toBe("agora");
    expect(formatWhen(Date.now() - 5 * 60_000, "pt-BR", labels)).toBe("5 min");
    expect(formatWhen(Date.now() - 3 * 3_600_000, "pt-BR", labels)).toBe("3 h");
  });

  it("vazio sem data", () => {
    expect(formatWhen(null, "pt-BR", labels)).toBe("");
  });
});
