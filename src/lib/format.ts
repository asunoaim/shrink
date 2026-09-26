// Display helpers. Sizes use 1 MB = 1024 × 1024 bytes, like Explorer and Discord.

const MIB = 1024 * 1024;

export const mbToBytes = (mb: number) => Math.round(mb * MIB);
export const bytesToMb = (bytes: number) => bytes / MIB;

export function fmtMb(bytes: number): string {
  const mb = bytesToMb(bytes);
  return mb >= 100 ? `${Math.round(mb)} MB` : `${mb.toFixed(1)} MB`;
}

/** m:ss.t */
export function fmtTime(sec: number): string {
  const tenths = Math.round(Math.max(0, sec) * 10);
  const m = Math.floor(tenths / 600);
  const s = (tenths % 600) / 10;
  return `${m}:${s.toFixed(1).padStart(4, "0")}`;
}
