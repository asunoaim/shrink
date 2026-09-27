import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, initialTracks, migrateTargetMb, parentDir, resolveOutDir } from "./settings";

const s = (patch = {}) => ({ ...DEFAULT_SETTINGS, ...patch });

describe("resolveOutDir", () => {
  it("asks by default", () => {
    expect(resolveOutDir(s(), "C:\\clips\\a.mp4", false)).toEqual({ kind: "ask", title: "Save clips to…" });
  });
  it("uses the recording's folder", () => {
    expect(resolveOutDir(s({ saveTo: "nextToOriginal" }), "C:\\clips\\a.mp4", false)).toEqual({ kind: "dir", dir: "C:\\clips" });
  });
  it("uses the drive root, not a bare drive letter", () => {
    expect(resolveOutDir(s({ saveTo: "nextToOriginal" }), "D:\\a.mp4", false)).toEqual({ kind: "dir", dir: "D:\\" });
  });
  it("uses the fixed folder while it exists", () => {
    expect(resolveOutDir(s({ saveTo: "folder", folder: "E:\\out" }), "C:\\a.mp4", true)).toEqual({ kind: "dir", dir: "E:\\out" });
  });
  it("asks, and says why, when the fixed folder is gone or unset", () => {
    const missing = { kind: "ask", title: "Your save folder is missing. Save clips to…" };
    expect(resolveOutDir(s({ saveTo: "folder", folder: "E:\\out" }), "C:\\a.mp4", false)).toEqual(missing);
    expect(resolveOutDir(s({ saveTo: "folder", folder: null }), "C:\\a.mp4", true)).toEqual(missing);
  });
});

describe("parentDir", () => {
  it("handles both slash kinds", () => {
    expect(parentDir("C:/clips/a b/c.mp4")).toBe("C:/clips/a b");
    expect(parentDir("\\\\nas\\share\\c.mp4")).toBe("\\\\nas\\share");
  });
});

describe("initialTracks", () => {
  it("first track", () => expect(initialTracks("first", [1], 3)).toEqual([0]));
  it("all tracks", () => expect(initialTracks("all", [], 3)).toEqual([0, 1, 2]));
  it("last used, keeping only tracks this clip has", () => expect(initialTracks("last", [1, 2, 4], 3)).toEqual([1, 2]));
  it("last used falls back to the first track", () => expect(initialTracks("last", [4], 2)).toEqual([0]));
  it("no audio means no tracks", () => {
    for (const a of ["first", "all", "last"] as const) expect(initialTracks(a, [0], 0)).toEqual([]);
  });
});

describe("migrateTargetMb", () => {
  it("takes the size 0.2 remembered", () => expect(migrateTargetMb(s(), "8")?.targetMb).toBe(8));
  it("ignores missing or broken values", () => {
    expect(migrateTargetMb(s(), null)).toBeNull();
    expect(migrateTargetMb(s(), "banana")).toBeNull();
    expect(migrateTargetMb(s(), "-4")).toBeNull();
  });
});
