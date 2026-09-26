import { describe, expect, it } from "vitest";
import type { ClipOutcome } from "./api";
import { clampTarget, doneOutputs, mergeRetry } from "./exporting";

const done = (output: string): ClipOutcome => ({ kind: "done", output, sizeBytes: 1 });
const failed: ClipOutcome = { kind: "failed", error: "disk full" };

describe("clampTarget", () => {
  it("pulls a remembered target down to what the section can use", () => {
    expect(clampTarget(40, 1, 14.2)).toBe(14);
  });
  it("keeps one decimal under 10 MB", () => {
    expect(clampTarget(9.87, 1, 9.87)).toBe(9.8);
  });
  it("raises a target below the minimum", () => {
    expect(clampTarget(0.5, 1.3, 50)).toBe(1.3);
  });
  it("leaves a valid target alone", () => {
    expect(clampTarget(25, 1, 50)).toBe(25);
  });
});

describe("mergeRetry", () => {
  it("replaces only the retried clips, by clip number", () => {
    const before = [done("a - clip 1.mp4"), failed, done("a - clip 3.mp4")];
    const after = mergeRetry(before, [2], [done("a - clip 2.mp4")]);
    expect(after).toEqual([done("a - clip 1.mp4"), done("a - clip 2.mp4"), done("a - clip 3.mp4")]);
  });
  it("lists every finished file for the clipboard", () => {
    expect(doneOutputs([done("x"), failed, done("y")])).toEqual(["x", "y"]);
  });
});
