// Export bookkeeping that doesn't touch the screen.

import type { ClipOutcome } from "./api";

/** Keep the size target inside the slider's range, rounded the way the slider shows it. */
export function clampTarget(mb: number, min: number, max: number): number {
  const v = Math.min(max, Math.max(min, mb));
  if (v === min) return v;
  return v < 10 ? Math.floor(v * 10) / 10 : Math.floor(v);
}

/** Outcomes after "Try again": the retried clip numbers (1-based) get their new results. */
export function mergeRetry(previous: ClipOutcome[], retried: number[], results: ClipOutcome[]): ClipOutcome[] {
  const merged = [...previous];
  retried.forEach((n, i) => {
    if (results[i]) merged[n - 1] = results[i];
  });
  return merged;
}

export function doneOutputs(outcomes: ClipOutcome[]): string[] {
  return outcomes.flatMap((o) => (o.kind === "done" ? [o.output] : []));
}
