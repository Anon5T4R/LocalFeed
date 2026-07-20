import { describe, expect, it } from "vitest";
import { MIN_QUERY, SEARCH_PERIODS, isSearchable, sinceMsFor } from "../search";

const DIA = 86_400_000;

describe("sinceMsFor", () => {
  it("'qualquer data' não corta nada", () => {
    expect(sinceMsFor("any")).toBeNull();
  });

  it("converte o período em instante de corte", () => {
    const agora = 1_000_000_000_000;
    expect(sinceMsFor("week", agora)).toBe(agora - 7 * DIA);
    expect(sinceMsFor("month", agora)).toBe(agora - 30 * DIA);
    expect(sinceMsFor("year", agora)).toBe(agora - 365 * DIA);
  });

  it("todo período da UI tem conversão (nenhum cai em undefined)", () => {
    for (const p of SEARCH_PERIODS) {
      const v = sinceMsFor(p, 1_000_000_000_000);
      expect(v === null || Number.isFinite(v)).toBe(true);
    }
  });
});

describe("isSearchable", () => {
  it("exige o mínimo de caracteres, ignorando espaços", () => {
    expect(isSearchable("")).toBe(false);
    expect(isSearchable("   ")).toBe(false);
    expect(isSearchable("a")).toBe(false);
    expect(isSearchable("  a  ")).toBe(false);
    expect(isSearchable("ab")).toBe(true);
    expect(isSearchable("polvilho")).toBe(true);
    expect("ab".length).toBe(MIN_QUERY);
  });
});
