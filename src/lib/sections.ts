// Highlight sections on the timeline. Pure functions: every change returns a new list.

export type Section = { id: number; start: number; end: number };
export type Edge = "start" | "end";
export type MarkState = { sections: Section[]; pendingIn: number | null; selected: number | null };

/** Shorter than this counts as a click, not a section. */
export const MIN_LEN = 0.1;
/** O without a pending I makes a section of this length ending at the playhead. */
const DEFAULT_LEN = 10;

const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));
const nextId = (list: Section[]) => list.reduce((m, s) => Math.max(m, s.id), 0) + 1;

export function addSection(list: Section[], a: number, b: number, duration: number): Section[] {
  const start = clamp(Math.min(a, b), 0, duration);
  const end = clamp(Math.max(a, b), 0, duration);
  if (end - start < MIN_LEN) return list;
  return [...list, { id: nextId(list), start, end }];
}

/** Sections in time order with their clip number (1, 2, 3 …). */
export function numbered(list: Section[]): (Section & { number: number })[] {
  return [...list].sort((a, b) => a.start - b.start).map((s, i) => ({ ...s, number: i + 1 }));
}

export function moveSection(list: Section[], id: number, delta: number, duration: number): Section[] {
  return list.map((s) => {
    if (s.id !== id) return s;
    const len = s.end - s.start;
    const start = clamp(s.start + delta, 0, duration - len);
    return { ...s, start, end: start + len };
  });
}

export function resizeSection(list: Section[], id: number, edge: Edge, t: number, duration: number): Section[] {
  return list.map((s) => {
    if (s.id !== id) return s;
    return edge === "start"
      ? { ...s, start: clamp(t, 0, s.end - MIN_LEN) }
      : { ...s, end: clamp(t, s.start + MIN_LEN, duration) };
  });
}

export function removeSection(list: Section[], id: number): Section[] {
  return list.filter((s) => s.id !== id);
}

export function longest(list: Section[]): number {
  return list.reduce((m, s) => Math.max(m, s.end - s.start), 0);
}

/** I key: move the selected section's start, or remember where a new one begins. */
export function markIn(st: MarkState, t: number, duration: number): MarkState {
  if (st.selected !== null && st.sections.some((s) => s.id === st.selected)) {
    return { ...st, sections: resizeSection(st.sections, st.selected, "start", t, duration) };
  }
  return { ...st, pendingIn: clamp(t, 0, duration) };
}

/** O key: move the selected section's end, or finish a new section at the playhead. */
export function markOut(st: MarkState, t: number, duration: number): MarkState {
  if (st.selected !== null && st.sections.some((s) => s.id === st.selected)) {
    return { ...st, sections: resizeSection(st.sections, st.selected, "end", t, duration) };
  }
  const from = st.pendingIn !== null && st.pendingIn < t ? st.pendingIn : t - DEFAULT_LEN;
  // the new section stays unselected, so the next I/O pair starts another one
  return { sections: addSection(st.sections, from, t, duration), pendingIn: null, selected: null };
}
