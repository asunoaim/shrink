import { describe, expect, it } from "vitest";
import { bytesToMb, fmtMb, fmtTime, mbToBytes, fmtClock, fmtFormat } from "./format";

describe("fmtTime", () => {
  it("shows minutes, seconds and tenths", () => {
    expect(fmtTime(36.44)).toBe("0:36.4");
    expect(fmtTime(70)).toBe("1:10.0");
    expect(fmtTime(3725.25)).toBe("62:05.3");
  });
  it("never shows negative time", () => {
    expect(fmtTime(-1)).toBe("0:00.0");
  });
});

describe("sizes", () => {
  it("1 MB is 1024 x 1024 bytes, like Explorer and Discord", () => {
    expect(mbToBytes(25)).toBe(26214400);
    expect(bytesToMb(26214400)).toBe(25);
  });
  it("formats with one decimal under 100 MB", () => {
    expect(fmtMb(mbToBytes(24.14))).toBe("24.1 MB");
    expect(fmtMb(mbToBytes(243))).toBe("243 MB");
  });
});

describe("fmtFormat", () => {
  it("always writes fps", () => {
    expect(fmtFormat({ height: 1080, fps: 119.88 })).toBe("1080p · 120 fps");
    expect(fmtFormat({ height: 720, fps: 60 })).toBe("720p · 60 fps");
  });
});

describe("fmtClock", () => {
  it("drops tenths on whole seconds", () => {
    expect(fmtClock(40)).toBe("0:40");
    expect(fmtClock(40.02)).toBe("0:40");
    expect(fmtClock(125)).toBe("2:05");
  });
  it("keeps tenths otherwise", () => {
    expect(fmtClock(39.5)).toBe("0:39.5");
  });
});
