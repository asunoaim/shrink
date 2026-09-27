import { describe, expect, it } from "vitest";
import { keyAction, type KeyContext } from "./keys";

const idle: KeyContext = { typing: false, buttonFocusedByKeyboard: false, exporting: false, shift: false };

describe("keyAction", () => {
  it("Q and E jump between highlights", () => {
    expect(keyAction("q", idle)).toEqual({ kind: "jump", dir: -1 });
    expect(keyAction("E", idle)).toEqual({ kind: "jump", dir: 1 });
  });
  it("never takes Tab", () => {
    expect(keyAction("Tab", idle)).toBeNull();
    expect(keyAction("Tab", { ...idle, shift: true })).toBeNull();
  });
  it("Space plays after a mouse click on a button", () => {
    expect(keyAction(" ", idle)).toEqual({ kind: "toggle" });
  });
  it("Space presses a button reached by keyboard", () => {
    expect(keyAction(" ", { ...idle, buttonFocusedByKeyboard: true })).toBeNull();
  });
  it("does nothing while typing", () => {
    for (const k of ["q", "e", "i", "o", " ", "Delete", "ArrowLeft"]) expect(keyAction(k, { ...idle, typing: true })).toBeNull();
  });
  it("arrows step a frame, with Shift a second", () => {
    expect(keyAction("ArrowRight", idle)).toEqual({ kind: "step", unit: "frame", dir: 1 });
    expect(keyAction("ArrowLeft", { ...idle, shift: true })).toEqual({ kind: "step", unit: "second", dir: -1 });
  });
  it("during export only playback and jumping work", () => {
    const ex = { ...idle, exporting: true };
    expect(keyAction("e", ex)).toEqual({ kind: "jump", dir: 1 });
    expect(keyAction(" ", ex)).toEqual({ kind: "toggle" });
    expect(keyAction("i", ex)).toBeNull();
    expect(keyAction("Delete", ex)).toBeNull();
  });
  it("marks, removes and clears", () => {
    expect(keyAction("i", idle)).toEqual({ kind: "markIn" });
    expect(keyAction("O", idle)).toEqual({ kind: "markOut" });
    expect(keyAction("Backspace", idle)).toEqual({ kind: "remove" });
    expect(keyAction("Escape", idle)).toEqual({ kind: "clear" });
  });
});
