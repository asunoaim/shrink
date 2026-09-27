// Every color, size and spacing comes from theme.css. Fixed geometry is allowed when marked.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const files = [
  "src/routes/+page.svelte",
  ...readdirSync("src/lib/components").map((f) => join("src/lib/components", f)),
].filter((f) => f.endsWith(".svelte"));

function styleLines(file: string): string[] {
  const src = readFileSync(file, "utf8");
  const m = src.match(/<style>([\s\S]*?)<\/style>/);
  return m ? m[1].split("\n") : [];
}

describe("design tokens", () => {
  for (const f of files) {
    it(`${f} has no literal colors`, () => {
      const bad = readFileSync(f, "utf8").split("\n").filter((l) => /#[0-9a-fA-F]{3,8}\b|rgba?\(/.test(l));
      expect(bad).toEqual([]);
    });
    it(`${f} has no loose px values`, () => {
      const bad = styleLines(f).filter((l) => {
        if (l.includes("/* geometry */")) return false;
        const px = l.match(/-?\d+(\.\d+)?px/g) ?? [];
        return px.some((v) => !["1px", "2px", "-1px", "-2px"].includes(v));
      });
      expect(bad).toEqual([]);
    });
    it(`${f} uses only loaded font weights`, () => {
      const bad = styleLines(f).filter((l) => /font-weight:\s*(700|800)/.test(l) && !l.includes("Sora wordmark"));
      expect(bad).toEqual([]);
    });
  }
});
