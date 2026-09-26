import { describe, expect, it } from "vitest";
import {
  addSection,
  longest,
  markIn,
  markOut,
  moveSection,
  neighbour,
  numbered,
  removeSection,
  resizeSection,
  type MarkState,
  type Section,
} from "./sections";

const D = 70;
const s = (id: number, start: number, end: number): Section => ({ id, start, end });

describe("addSection", () => {
  it("adds a section with a fresh id", () => {
    const list = addSection([s(1, 0, 5)], 10, 20, D);
    expect(list).toHaveLength(2);
    expect(list[1]).toEqual({ id: 2, start: 10, end: 20 });
  });
  it("swaps a section drawn right-to-left", () => {
    expect(addSection([], 20, 10, D)[0]).toMatchObject({ start: 10, end: 20 });
  });
  it("clamps to the clip", () => {
    expect(addSection([], -5, 99, D)[0]).toMatchObject({ start: 0, end: 70 });
  });
  it("ignores a click without a drag (too short)", () => {
    expect(addSection([], 10, 10.05, D)).toEqual([]);
  });
});

describe("numbered", () => {
  it("numbers sections by start time", () => {
    const n = numbered([s(1, 40, 50), s(2, 5, 9), s(3, 20, 25)]);
    expect(n.map((x) => [x.id, x.number])).toEqual([
      [2, 1],
      [3, 2],
      [1, 3],
    ]);
  });
});

describe("moveSection", () => {
  it("keeps the length while moving", () => {
    expect(moveSection([s(1, 10, 20)], 1, 5, D)[0]).toMatchObject({ start: 15, end: 25 });
  });
  it("stops at the clip edges", () => {
    expect(moveSection([s(1, 10, 20)], 1, -30, D)[0]).toMatchObject({ start: 0, end: 10 });
    expect(moveSection([s(1, 10, 20)], 1, 80, D)[0]).toMatchObject({ start: 60, end: 70 });
  });
});

describe("resizeSection", () => {
  it("moves one edge", () => {
    expect(resizeSection([s(1, 10, 20)], 1, "start", 12, D)[0]).toMatchObject({ start: 12, end: 20 });
    expect(resizeSection([s(1, 10, 20)], 1, "end", 30, D)[0]).toMatchObject({ start: 10, end: 30 });
  });
  it("never lets the edges cross", () => {
    const r = resizeSection([s(1, 10, 20)], 1, "start", 25, D)[0];
    expect(r.end - r.start).toBeGreaterThan(0);
    expect(r.end).toBe(20);
  });
});

describe("markIn / markOut", () => {
  it("I then O creates a section", () => {
    let st = markIn({ sections: [], pendingIn: null, selected: null }, 12, D);
    expect(st.pendingIn).toBe(12);
    st = markOut(st, 18, D);
    expect(st.sections[0]).toMatchObject({ start: 12, end: 18 });
    expect(st.pendingIn).toBeNull();
  });
  it("a second I/O pair makes a second section", () => {
    let st: MarkState = { sections: [], pendingIn: null, selected: null };
    st = markOut(markIn(st, 12, D), 21, D);
    st = markOut(markIn(st, 41, D), 47, D);
    expect(st.sections.map((x) => [x.start, x.end])).toEqual([
      [12, 21],
      [41, 47],
    ]);
  });
  it("O without I ends a section at the playhead, starting 10 s earlier", () => {
    const st = markOut({ sections: [], pendingIn: null, selected: null }, 30, D);
    expect(st.sections[0]).toMatchObject({ start: 20, end: 30 });
  });
  it("with a section selected, I and O move its edges", () => {
    let st: MarkState = { sections: [s(1, 10, 20)], pendingIn: null, selected: 1 };
    st = markIn(st, 12, D);
    st = markOut(st, 25, D);
    expect(st.sections[0]).toMatchObject({ start: 12, end: 25 });
  });
});

describe("removeSection / longest", () => {
  it("removes by id", () => {
    expect(removeSection([s(1, 0, 5), s(2, 6, 9)], 1)).toEqual([s(2, 6, 9)]);
  });
  it("finds the longest duration", () => {
    expect(longest([s(1, 0, 5), s(2, 6, 16)])).toBe(10);
    expect(longest([])).toBe(0);
  });
});

describe("neighbour (Tab / Shift+Tab)", () => {
  const list = [s(1, 40, 50), s(2, 5, 9), s(3, 20, 25)];
  it("goes to the next section in time order", () => {
    expect(neighbour(list, 2, 1)?.id).toBe(3);
    expect(neighbour(list, 3, 1)?.id).toBe(1);
  });
  it("goes back with Shift+Tab", () => {
    expect(neighbour(list, 1, -1)?.id).toBe(3);
  });
  it("wraps around at the ends", () => {
    expect(neighbour(list, 1, 1)?.id).toBe(2);
    expect(neighbour(list, 2, -1)?.id).toBe(1);
  });
  it("starts at the first or last when nothing is selected", () => {
    expect(neighbour(list, null, 1)?.id).toBe(2);
    expect(neighbour(list, null, -1)?.id).toBe(1);
  });
  it("is null without sections", () => {
    expect(neighbour([], null, 1)).toBeNull();
  });
});
