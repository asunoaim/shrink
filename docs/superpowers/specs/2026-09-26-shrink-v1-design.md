# shrink v1 — design spec

Date: 2026-09-26 · Status: awaiting review · Name "shrink" is a placeholder

## 1. Purpose

A small, good-looking Windows app for gameplay clips (KovaaK's, Call of Duty, other shooters) with two jobs:

1. **Cut highlights for montages.** Mark several sections in one recording and save each as its own file, losslessly and in seconds.
2. **Shrink clips for sharing.** Save sections at a size you choose (for example 25 MB), with the best quality that fits.

It replaces the Avidemux + `discord-clip.ps1` workflow. It is built for David first and designed to grow into a general tool that others can download from GitHub.

**v1 is done when** going from a fresh clip to a montage file or a Discord-ready file is faster and easier than with Avidemux plus the .ps1 script, with no visible quality loss at sensible sizes.

## 2. Words used in this spec

| Word | Meaning |
|---|---|
| Trim | Make a clip shorter by keeping only a section of it |
| Section | One highlight marked on the timeline; each section becomes one output file |
| Keyframe | A full picture stored inside the video every so often; lossless cuts can only start on one |
| Original quality | Export without re-encoding: instant and lossless |
| Shrink | Export with re-encoding to hit a target file size |
| Crop | Cut away picture edges; **not in v1** |

## 3. What v1 does

### 3.1 Opening a clip
- Drag and drop onto the window, **"Open in shrink"** in Explorer's right-click menu, or an **Open file…** button.
- One clip at a time. Any format ffmpeg can read (mp4, mkv, mov, …).
- The empty start screen says "Drop a clip here" and shows the right-click hint and the Open button.
- Windows 11 note: classic right-click entries appear under "Show more options" (the same place as the old SendTo entry). Getting into the compact top-level menu needs app packaging and is deferred.

### 3.2 Preview
- The original file plays directly in the app's preview whenever the PC can decode it.
- If direct playback fails (for example HEVC without system support), the app makes a lightweight **preview copy** in the background and plays that instead. Exports always use the original file.
- The timeline shows a strip of thumbnails generated from the clip.

### 3.3 Marking sections
- **Mouse:** drag across the timeline to draw a section; drag its edges to adjust; drag the whole section to move it.
- **Keyboard:** `I` sets a start at the playhead, `O` sets the end.
- Select a section and press `Delete` to remove it.
- Moving through the clip: `Space` play/pause, `←`/`→` one frame, `Shift+←`/`→` one second.
- Sections are numbered in timeline order (1, 2, 3 …). Each becomes its own file.

### 3.4 Export bar
One floating bar at the bottom of the window contains:
- **Mode toggle:** `Original` | `Shrink`. The choice applies to every section in this export.
- **Size slider** (only in Shrink mode): target size per clip in MB.
  - It goes from 1 MB up to the longest section's size in original quality; a bigger target couldn't improve anything.
  - It starts at 25 MB and afterwards remembers the last value used.
  - Clicking the number lets you type an exact value.
- **Audio checkboxes:** `Game` / `Mic`. Shown when the clip has separate audio tracks; otherwise the audio is kept as recorded.
- **Export button:** "Export 3 clips".

### 3.5 Original quality export
- Stream copy (no re-encoding) in the source's container format.
- Each file starts at the **keyframe at or before** the section's start and ends at the section's end. The timeline shows a small marker at the real start ("actual start 0:39.2") so there are no surprises.
- Checked audio tracks are kept as **separate tracks**, so a video editor still sees game and mic apart.

### 3.6 Shrink export
- Output: **H.264 in .mp4** (plays inline in Discord and nearly everywhere), `faststart` enabled so it streams immediately.
- Cuts are frame-exact (the video is re-encoded anyway).
- **Size math:** `video bitrate = (target size × 8 ÷ duration) − audio bitrate − 3 % safety margin`. Audio is AAC 160 kbps stereo.
- **The target is a promise:** if an output still exceeds the target, it is re-encoded automatically at a proportionally lower bitrate (up to 2 retries). No output is ever larger than the target.
- Checked audio tracks are **mixed into one track**, because Discord only plays the first track.
- Resolution and fps stay as recorded unless the user picks a fix from the warning (3.7).
- **Encoder choice** (detected at startup by a short test encode): NVIDIA NVENC → AMD AMF → Intel Quick Sync → x264 on the CPU. When the CPU is used, the export bar shows a small "slower (CPU)" note.

### 3.7 "Too small" warning
- Measured as **bits per pixel per frame**: `video bitrate ÷ (width × height × fps)`.
- Starting thresholds, taken from the `discord-clip.ps1` VMAF tests (1080p120, H.264 NVENC): about 0.048 looked like the original (VMAF 96), and about 0.033 was where visible loss started (VMAF 94.5).
  - **Below 0.035:** warn.
  - **At or above 0.045:** counts as "clean" when suggesting fixes.
  - Both values will be tuned with real clips during the build.
- The slider track shows the too-small range as a **yellow zone**. It is recomputed for each clip, since longer clips need more MB.
- While the slider is inside the zone, **one extra line appears inside the export bar under the slider**, for example "! Tight for 1440p · 120 fps — will look blocky", with one-click fixes such as `Use 720p · 60 (clean)` and `1080p · 60`. The suggested fix is the highest resolution/fps combination that reaches the "clean" threshold.
- It is not a popup and never blocks. Export works either way, and the line disappears when the slider leaves the zone.

### 3.8 Saving and finishing
- On every export a **folder picker** opens once, however many sections there are.
- File names: `<original name> - clip 1.mp4`, `- clip 2`, …. If a name already exists, a number is added; **nothing is ever overwritten**.
- **Progress** appears inside the export bar ("Clip 2 of 3 · shrinking to 25 MB · 58 % · ~14 s left") with a **Cancel** button. The preview stays usable while exporting.
- **Done:** "✓ 3 clips saved · 24.1 · 23.8 · 24.6 MB · copied to clipboard", with **Show in folder** and **Done** buttons. Explorer does not open by itself.
- The finished files are **copied to the clipboard as files**, so Ctrl+V in Discord works right away.

### 3.9 When things go wrong
| Situation | Behavior |
|---|---|
| Clip can't be opened | Clear message on the start screen; the app keeps running |
| One clip fails during export | That clip is marked failed, the others still finish, and a "Try again" button is offered |
| Cancel | The half-written file is deleted; clips that already finished are kept |
| Name already exists | A number is added; nothing is overwritten |
| No usable GPU encoder | Falls back to CPU with a "slower (CPU)" note |
| Clipboard | Only successfully finished files are copied |

## 4. Look and feel

Style **"Paper & Ember — Dusk"**: warm dark mode, never pure black, with one burnt-orange accent.

| Token | Value |
|---|---|
| Background | `#2b2622` |
| Panel | `#342e29` |
| Panel, raised / tracks | `#403832` |
| Lines | `#4a4139` |
| Text | `#f3ebdf` |
| Muted text | `#a8998a` |
| Accent (ember) | `#ef6a3f` |
| Warning | `#f2b640` |

- Fonts: **Fraunces** (serif) for the "shrink" logo only, **Inter** for everything else.
- Pill-shaped controls, 12–16 px corner radius, and a soft ember glow on the main button and the logo dot.
- Layout: title bar (logo, file name, resolution/fps/size) → video preview → timeline with numbered sections → floating export bar.
- Reference mockups: `.superpowers/brainstorm/…/ember-dark.html` (option A, Dusk), `export-flow.html` and `inline-warning.html` (option B). These are local only and not in git.

## 5. How it's built

- **Tauri 2** desktop app: the screen is Svelte + TypeScript, and the engine is Rust.
- **ffmpeg + ffprobe** ship inside the app. The app is open source, so a GPL build with x264 can be bundled.

The engine is split into small parts:

| Part | Job |
|---|---|
| Clip reader | Duration, resolution, fps, codecs, audio tracks and keyframe times (via ffprobe) |
| Planner | Turns sections + settings into exact export jobs: size math, keyframe starts, audio mapping, file names. Pure logic with no side effects, so it is fully unit-tested |
| Encoder check | Finds the best working encoder at startup |
| Runner | Runs the jobs one after another, reports progress, handles Cancel and cleanup |
| Preview helper | Timeline thumbnails and the preview-copy fallback |
| Finish | Clipboard copy and Show in folder |

- The screen talks to the engine only through a small set of commands (open clip, plan export, run export, cancel) and progress events.
- **App size:** the app itself is about 10 MB, and a standard ffmpeg build adds roughly 50–100 MB. v1 uses a standard build. A slimmed-down custom ffmpeg is planned before the first public release.
- **Installer:** a normal Windows installer (built by Tauri). It also registers the "Open in shrink" right-click entry for video files.

## 6. Testing

- **Unit tests (planner):** size math, keyframe start selection, the too-small threshold and suggested fixes, audio mapping, file naming and name collisions.
- **Integration tests:** the tests generate tiny synthetic clips with ffmpeg (separate audio tracks, known keyframes) and check:
  - shrink outputs are never over the target
  - durations are correct
  - the audio track layout is correct
  - Original-quality output is lossless (copied packets match the source)
- **Real-clip checks (local only, never committed):**
  - OBS 1440p120 HEVC clips and CoD clips
  - a speed comparison against HandBrake and `discord-clip.ps1` on the same clip
  - a VMAF quality check against the script's results
- **First check of the build:** does HEVC play directly in the Tauri preview on David's PC? The preview design depends on the answer.
- **CI:** later, GitHub Actions runs the tests on every push.
- **Milestone hand-offs:** a working build for David plus a short "try this" list. Claude can also launch the app and take screenshots.

## 7. Not in v1 (planned later)

**Options menu:**
- save location (ask each time / next to the original / fixed folder)
- cut mode (free start with the actual-start marker / snap to keyframes / smart cut for frame-exact lossless cuts)
- automatic quality adjustment instead of the warning
- a manual resolution/fps override

**Features:**
- a recent clips list that watches clip folders (OBS, CoD, …)
- cropping the picture (for example vertical 9:16)
- several clips at once
- Mac and Linux builds
- the compact Windows 11 right-click menu entry

## 8. Outside the app

- **OBS audio tracks:**
  - track 1 = full mix (for Discord)
  - track 2 = game only
  - track 3 = mic only

  Right now every source goes to all three tracks, so the tracks are identical. The change needs a backup of the OBS profile first, and David has to exit OBS from the tray. It only affects new recordings.
- **Before public release:**
  - choose the final name
  - pick the open-source license (GPL-3.0, to match the bundled GPL ffmpeg)
  - include the ffmpeg credits and license text
