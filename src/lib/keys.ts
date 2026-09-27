// Keyboard shortcuts as data, so the rules can be tested without a screen.

export type KeyContext = {
  /** focus is in a text, number or range input */
  typing: boolean;
  /** a button has focus and got it from the keyboard (:focus-visible) */
  buttonFocusedByKeyboard: boolean;
  exporting: boolean;
  shift: boolean;
};

export type KeyAction =
  | { kind: "jump"; dir: 1 | -1 }
  | { kind: "toggle" }
  | { kind: "step"; unit: "frame" | "second"; dir: 1 | -1 }
  | { kind: "markIn" }
  | { kind: "markOut" }
  | { kind: "remove" }
  | { kind: "clear" };

export function keyAction(key: string, ctx: KeyContext): KeyAction | null {
  if (ctx.typing) return null;
  const k = key.length === 1 ? key.toLowerCase() : key;
  let action: KeyAction | null = null;
  switch (k) {
    case "q":
      action = { kind: "jump", dir: -1 };
      break;
    case "e":
      action = { kind: "jump", dir: 1 };
      break;
    case " ":
      action = ctx.buttonFocusedByKeyboard ? null : { kind: "toggle" };
      break;
    case "ArrowLeft":
    case "ArrowRight":
      action = { kind: "step", unit: ctx.shift ? "second" : "frame", dir: k === "ArrowLeft" ? -1 : 1 };
      break;
    case "i":
      action = { kind: "markIn" };
      break;
    case "o":
      action = { kind: "markOut" };
      break;
    case "Delete":
    case "Backspace":
      action = { kind: "remove" };
      break;
    case "Escape":
      action = { kind: "clear" };
      break;
  }
  // while exporting, the preview stays usable but the highlights are locked
  if (ctx.exporting && action && !["jump", "toggle", "step"].includes(action.kind)) return null;
  return action;
}
