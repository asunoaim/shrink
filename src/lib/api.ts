// Typed access to the Rust engine (src-tauri/src/commands.rs). Field names mirror its serde output.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Encoder = "nvenc" | "amf" | "qsv" | "x264";
export type AudioTrack = { index: number; channels: number; title: string | null };
export type Format = { width: number; height: number; fps: number };

export type ClipInfo = {
  path: string;
  formatName: string;
  containerExt: string;
  duration: number;
  sizeBytes: number;
  bitrate: number;
  width: number;
  height: number;
  fps: number;
  videoCodec: string;
  audioTracks: AudioTrack[];
  keyframes: number[];
};

export type ClipView = { info: ClipInfo; thumbs: string | null; thumbCount: number; encoder: Encoder };

export type Suggestion = { format: Format; bpp: number; clean: boolean; suggested: boolean };
export type SizeAdvice = {
  sliderMinBytes: number;
  sliderMaxBytes: number;
  warnMaxBytes: number;
  tooSmall: boolean;
  suggestions: Suggestion[];
};

export type Mode = { kind: "original" } | { kind: "shrink"; targetBytes: number; format: Format | null };
export type ExportRequest = {
  sections: { start: number; end: number; number: number }[];
  mode: Mode;
  audioTracks: number[];
  outDir: string;
};

export type ClipOutcome =
  | { kind: "done"; output: string; sizeBytes: number }
  | { kind: "failed"; error: string }
  | { kind: "cancelled" };

export type RunEvent =
  | { kind: "clipStarted"; clipNumber: number; clipCount: number }
  | { kind: "progress"; clipNumber: number; fraction: number }
  | { kind: "clipFinished"; clipNumber: number; outcome: ClipOutcome };

export type ExportDone = { outcomes: ClipOutcome[]; copiedToClipboard: boolean };

export const openClip = (path: string) => invoke<ClipView>("open_clip", { path });
export const sizeAdvice = (longest: number, targetBytes: number, hasAudio: boolean) =>
  invoke<SizeAdvice>("size_advice", { longest, targetBytes, hasAudio });
export const startExport = (request: ExportRequest) => invoke<unknown[]>("start_export", { request });
export const cancelExport = () => invoke<void>("cancel_export");
export const makeProxy = () => invoke<string>("make_proxy");
export const initialFile = () => invoke<string | null>("initial_file");
export const copyToClipboard = (paths: string[]) => invoke<void>("copy_to_clipboard", { paths });

export const onExportEvent = (f: (e: RunEvent) => void): Promise<UnlistenFn> =>
  listen<RunEvent>("export-event", (e) => f(e.payload));
export const onExportDone = (f: (e: ExportDone) => void): Promise<UnlistenFn> =>
  listen<ExportDone>("export-done", (e) => f(e.payload));
export const onProxyProgress = (f: (fraction: number) => void): Promise<UnlistenFn> =>
  listen<number>("proxy-progress", (e) => f(e.payload));

export const fileUrl = (path: string) => convertFileSrc(path);

/** "Replay 2026-09-26 00-28-31.mp4" from a full Windows path. */
export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
