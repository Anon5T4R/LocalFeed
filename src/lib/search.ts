import type { SearchPeriod } from "./types";

export const SEARCH_PERIODS: SearchPeriod[] = ["any", "week", "month", "year"];

const DAYS: Record<Exclude<SearchPeriod, "any">, number> = {
  week: 7,
  month: 30,
  year: 365,
};

/**
 * Período → instante de corte pro backend (null = sem corte).
 * `now` entra por parâmetro pra dar pra testar sem mexer no relógio.
 */
export function sinceMsFor(period: SearchPeriod, now = Date.now()): number | null {
  if (period === "any") return null;
  return now - DAYS[period] * 86_400_000;
}

/** Consulta curta demais não vale ida ao backend (e devolve o mundo inteiro). */
export const MIN_QUERY = 2;

export function isSearchable(query: string): boolean {
  return query.trim().length >= MIN_QUERY;
}
