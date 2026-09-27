// Settings helpers that don't touch the screen or the disk.

import type { AudioDefault, Settings } from "./api";

export const DEFAULT_SETTINGS: Settings = {
  saveTo: "ask",
  folder: null,
  copyToClipboard: true,
  audio: "first",
  lastAudio: [0],
  startMode: "original",
  targetMb: 25,
};

export type OutDir = { kind: "ask"; title: string } | { kind: "dir"; dir: string };

/** "C:\clips\a.mp4" → "C:\clips"; a drive root keeps its slash ("D:\"), or Windows reads "D:" as "current folder on D". */
export function parentDir(path: string): string {
  const i = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  const dir = i < 0 ? "" : path.slice(0, i);
  return /^[A-Za-z]:$/.test(dir) ? `${dir}${path[i]}` : dir;
}

/** Where this export goes. `folderExists` is about `s.folder`, checked by the caller. */
export function resolveOutDir(s: Settings, clipPath: string, folderExists: boolean): OutDir {
  if (s.saveTo === "nextToOriginal") return { kind: "dir", dir: parentDir(clipPath) };
  if (s.saveTo === "folder") {
    if (s.folder && folderExists) return { kind: "dir", dir: s.folder };
    return { kind: "ask", title: "Your save folder is missing. Save clips to…" };
  }
  return { kind: "ask", title: "Save clips to…" };
}

/** Audio tracks ticked when a clip opens. */
export function initialTracks(audio: AudioDefault, lastAudio: number[], trackCount: number): number[] {
  if (trackCount === 0) return [];
  if (audio === "all") return Array.from({ length: trackCount }, (_, i) => i);
  if (audio === "last") {
    const kept = lastAudio.filter((t) => t < trackCount);
    return kept.length > 0 ? kept : [0];
  }
  return [0];
}

/** 0.2 kept the size in browser storage; returns updated settings, or null when there's nothing to take over. */
export function migrateTargetMb(s: Settings, stored: string | null): Settings | null {
  if (stored === null) return null;
  const v = Number(stored);
  if (!Number.isFinite(v) || v <= 0) return null;
  return { ...s, targetMb: Math.min(1000, Math.max(1, v)) };
}
