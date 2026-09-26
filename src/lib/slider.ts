// Size slider scale and keyframe helpers.

/** Slider position (0..1) for a value on a logarithmic min..max scale. */
export function valueToPos(v: number, min: number, max: number): number {
  if (max <= min) return 1;
  const p = Math.log(v / min) / Math.log(max / min);
  return Math.min(1, Math.max(0, p));
}

/** Value for a slider position (0..1) on a logarithmic min..max scale. */
export function posToValue(p: number, min: number, max: number): number {
  if (max <= min) return max;
  return min * Math.pow(max / min, Math.min(1, Math.max(0, p)));
}

/** Where a lossless export really starts: the keyframe at or before `start` (mirrors the engine). */
export function actualStart(keyframes: number[], start: number): number {
  let best = 0;
  for (const k of keyframes) {
    if (k <= start + 1e-3) best = k;
    else break;
  }
  return best;
}
