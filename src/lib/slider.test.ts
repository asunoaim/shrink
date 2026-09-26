import { describe, expect, it } from "vitest";
import { actualStart, posToValue, valueToPos } from "./slider";

describe("log slider", () => {
  it("maps the ends to min and max", () => {
    expect(posToValue(0, 1, 500)).toBeCloseTo(1);
    expect(posToValue(1, 1, 500)).toBeCloseTo(500);
  });
  it("round-trips", () => {
    for (const v of [1, 7.5, 25, 250, 500]) {
      expect(posToValue(valueToPos(v, 1, 500), 1, 500)).toBeCloseTo(v);
    }
  });
  it("gives small sizes more room than a linear slider", () => {
    // 25 MB sits past the middle of 1..500 on a log scale
    expect(valueToPos(25, 1, 500)).toBeGreaterThan(0.5);
  });
  it("clamps out-of-range values", () => {
    expect(valueToPos(0.2, 1, 500)).toBe(0);
    expect(valueToPos(900, 1, 500)).toBe(1);
  });
  it("survives min == max", () => {
    expect(valueToPos(5, 5, 5)).toBe(1);
    expect(posToValue(0.3, 5, 5)).toBe(5);
  });
});

describe("actualStart", () => {
  const k = [0, 1, 2, 3, 4];
  it("is the keyframe at or before the start", () => {
    expect(actualStart(k, 2.6)).toBe(2);
    expect(actualStart(k, 3)).toBe(3);
    expect(actualStart([], 3)).toBe(0);
  });
});
