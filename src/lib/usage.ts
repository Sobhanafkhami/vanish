import type { Subscription } from "./api";

export function fmtBytes(b: number) {
  const u = ["B", "KB", "MB", "GB", "TB", "PB"];
  let i = 0;
  while (b >= 1024 && i < u.length - 1) {
    b /= 1024;
    i++;
  }
  return `${b.toFixed(i > 1 ? 2 : i === 1 ? 1 : 0)} ${u[i]}`;
}

export type UsageKind =
  /** Total and used (or remaining) are both known. */
  | "full"
  /** Only the remaining balance is published; the total is estimated from the highest balance seen. */
  | "remaining"
  | "unlimited"
  /** Used is known but no total. */
  | "used"
  | "none";

export interface UsageView {
  kind: UsageKind;
  used: number | null;
  total: number | null;
  remaining: number | null;
  /** Share of the plan still available, 0-100 (null when it cannot be known). */
  remainingPct: number | null;
  daysLeft: number | null;
  expired: boolean;
}

/**
 * Normalises the different shapes panels report (header with total/used, remaining-only remarks, ...)
 * into one view the progress bar can draw.
 */
export function subUsage(s: Subscription): UsageView {
  const hasUsed = s.upload !== null || s.download !== null;
  const used = hasUsed ? (s.upload ?? 0) + (s.download ?? 0) : null;

  let daysLeft: number | null = null;
  let expired = false;
  if (s.expire) {
    const d = Math.ceil((s.expire * 1000 - Date.now()) / 86_400_000);
    expired = d < 0;
    daysLeft = Math.max(d, 0);
  }
  const base = { daysLeft, expired };
  const pct = (remaining: number, total: number) => Math.max(0, Math.min(100, (remaining / total) * 100));

  if (s.total === 0 && (used !== null || s.info_source)) {
    return { ...base, kind: "unlimited", used, total: 0, remaining: null, remainingPct: 100 };
  }
  if (s.total && s.total > 0 && used !== null) {
    const remaining = Math.max(s.total - used, 0);
    return { ...base, kind: "full", used, total: s.total, remaining, remainingPct: pct(remaining, s.total) };
  }
  if (s.total && s.total > 0 && s.remaining !== null) {
    return { ...base, kind: "full", used: Math.max(s.total - s.remaining, 0), total: s.total, remaining: s.remaining, remainingPct: pct(s.remaining, s.total) };
  }
  if (s.remaining !== null) {
    const peak = Math.max(s.remaining_peak ?? 0, s.remaining);
    return {
      ...base,
      kind: "remaining",
      used: peak > s.remaining ? peak - s.remaining : null,
      total: null,
      remaining: s.remaining,
      remainingPct: peak > 0 ? pct(s.remaining, peak) : null,
    };
  }
  if (used !== null) return { ...base, kind: "used", used, total: null, remaining: null, remainingPct: null };
  return { ...base, kind: "none", used: null, total: null, remaining: null, remainingPct: null };
}

/** Bar colour by how much is left. */
export function usageTone(remainingPct: number | null) {
  if (remainingPct === null) return "bg-white/30";
  return remainingPct > 25 ? "bg-accent" : remainingPct > 10 ? "bg-yellow-300" : "bg-red-400";
}
