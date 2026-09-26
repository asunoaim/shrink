# Build log — shrink v1 (night of 2026-09-26)

Every task result and every decision Claude made while building without review gates. Copied from the plan's ledger before its scratch workspace was deleted.

```
# SDD ledger — plan: docs/superpowers/plans/2026-09-26-shrink-v1.md
Branch: v1 (from main 2026-09-26). Executor: inline (executing-plans), David asleep and asked to proceed without review gates.
Pre-flight: interfaces checked. Task 6 consumes ClipInfo (T3), Format (T4), Encoder (T5); Task 7 consumes ExportJob::args (T6); Task 8 consumes JobKind::Encode{gop,target_bytes:Option} (T6) and runner (T7). Consistent.
Ruling: plan carries interfaces + test cases, not full code — David's budget; code lives in commits — cost if wrong: executor has to design details per task (me, same session).
Task 1: complete (commits 33fb7ed..c88cb0e, tests: cargo test --manifest-path src-tauri/Cargo.toml → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 2: complete (commits c88cb0e..decfc98, tests: cargo test --manifest-path src-tauri/Cargo.toml → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 3: Ruling: track titles read from title, then name (mp4), then non-generic handler_name — ffmpeg's mp4 muxer writes 'name', not 'title' — cost if wrong: generic labels 'Track N' in the UI
Task 3: complete (commits decfc98..a8c7958, tests: cargo test --manifest-path src-tauri/Cargo.toml --test probe → test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s)
Task 4: Ruling: 1 MB = 1 MiB (1,048,576 bytes) — Explorer and Discord count this way, so "25 MB" in the app never shows up bigger elsewhere — cost if wrong: targets ~5% smaller than a decimal reading
Task 4: Ruling: slider_max_bytes takes (source_bytes, source_duration, longest) instead of &ClipInfo — keeps it pure — cost if wrong: none
Task 4: Ruling: fps candidates = source fps and 60; 30 only when the source is <= 60 fps — a 30 fps suggestion for 120 fps gameplay is never what David wants — cost if wrong: one fewer option
Task 4: complete (commits a8c7958..5dae7da, tests: cargo test --manifest-path src-tauri/Cargo.toml --lib → test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 5: complete (commits 5dae7da..e648874, tests: cargo test --manifest-path src-tauri/Cargo.toml --test encoder → test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s)
Task 6: Ruling: Original clips end at the section end even though they start earlier at the keyframe — spec 3.5 — cost if wrong: none
Task 6: complete (commits e648874..fe00acb, tests: cargo test --manifest-path src-tauri/Cargo.toml --lib → test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 7: Ruling: when a GPU encoder is still over target after 2 retries, re-encode with x264 (2 more tries) before failing — NVENC floors at ~400 kbps on detailed 640x360@60 content and overshoots ~10% at 1080p; spec 3.6 makes the target a promise — cost if wrong: rare extra slow CPU encodes for very small targets
Task 7: Ruling: lossless copies may run up to ~0.1 s past the section end (whole packets, B-frame delay); test tolerance 0.15 s — cost if wrong: a few extra frames at the end
Task 7: complete (commits fe00acb..24d041f, tests: cargo test --manifest-path src-tauri/Cargo.toml --test runner → test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.52s)
Task 8: complete (commits 24d041f..e7a4654, tests: cargo test --manifest-path src-tauri/Cargo.toml → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 9: complete (commits e7a4654..e54dba7, tests: cargo test --manifest-path src-tauri/Cargo.toml --lib → test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 10-14: Ruling: native Windows title bar instead of the mockup's custom one — less risk, same look inside — cost if wrong: a custom title bar later
Task 10-14: Ruling: default audio = track 1 only when a clip has >= 2 tracks — OBS today writes 3 identical mixes; with the planned OBS layout track 1 = full mix — cost if wrong: one click to change
Task 10-14: Ruling: I/O makes a new section unless the user clicked one; mouse-drawn sections are selected — found live: selecting after O made the next I/O edit the old section
Task 10-14: Ruling: slider is logarithmic between min (100 kbps video) and max (longest section at source bitrate) — keeps 1-500 MB usable
Task 10-14: Ruling: NVENC switched from VBR p5 (+lookahead, temporal AQ) to CBR p3 multipass fullres, bufsize = 0.5 s — measured on clip.mp4: VBR overshot 10-74 % on 8 s clips, CBR within 2 %; p3 VMAF 88.78 vs p5 88.80 at 2.5x speed — cost if wrong: CBR spends bits evenly (slightly less efficient on static scenes)
Task 10-14: Ruling: AMF/QSV switched to CBR too, untested (no such GPU here) — cost if wrong: retries or the x264 fallback on those PCs
Task 10: complete (commit 0f04901, tests: npx vitest run → 25 passed; npm run check → 0 errors; manual run in tauri dev: open HEVC clip, I/O sections, shrink 4 MB with 720p60 fix, export 2 clips 3.9/3.9 MB, clipboard copied)
Task 11: complete (commit 0f04901, tests: npx vitest run → 25 passed; npm run check → 0 errors; manual run in tauri dev: open HEVC clip, I/O sections, shrink 4 MB with 720p60 fix, export 2 clips 3.9/3.9 MB, clipboard copied)
Task 12: complete (commit 0f04901, tests: npx vitest run → 25 passed; npm run check → 0 errors; manual run in tauri dev: open HEVC clip, I/O sections, shrink 4 MB with 720p60 fix, export 2 clips 3.9/3.9 MB, clipboard copied)
Task 13: complete (commit 0f04901, tests: npx vitest run → 25 passed; npm run check → 0 errors; manual run in tauri dev: open HEVC clip, I/O sections, shrink 4 MB with 720p60 fix, export 2 clips 3.9/3.9 MB, clipboard copied)
Task 14: complete (commit 0f04901, tests: npx vitest run → 25 passed; npm run check → 0 errors; manual run in tauri dev: open HEVC clip, I/O sections, shrink 4 MB with 720p60 fix, export 2 clips 3.9/3.9 MB, clipboard copied)
Task 15: complete (commits 0f04901..4d70d8e, tests: cargo test tools → pass; npx tauri build → shrink_0.1.0_x64-setup.exe 59 MB; silent install /S → files in %LOCALAPPDATA%\shrink incl. ffmpeg\, HKCU .mp4 shell\shrink command registered; installed exe opened MW3 1440p clip from the command line)
Task 16: skipped — needs David: GitHub repo creation/push, CI, HandBrake benchmark
Task 15: Ruling: installed the built installer for David's user (silent, per-user) to verify it end to end — reversible via Settings > Apps — cost if wrong: he uninstalls
Final review: opus reviewer, 0 Critical / 4 Important / 8 Minor
Final: fixed stale target bigger than the section — shrink_target_never_exceeds_the_sections_original_size + clampTarget tests RED→GREEN, suite 70 rust / 31 ts
Final: fixed ffmpeg outliving the app — ffmpeg_is_killed_when_its_job_object_closes RED→GREEN (killed processes report exit 0, so the test checks timing + still-running control), suite green
Final: fixed overwrite race / concurrent exports — a_file_that_appears_after_planning_is_never_overwritten RED→GREEN; start_export refuses while running; UI guards the folder dialog; suite green
Final: fixed Try again renumbering — retried_sections_keep_their_clip_number + mergeRetry tests RED→GREEN, suite green
Final: fixed stray x.png committed at repo root (my screenshot) — removed
Final: minor (deferred): proxy cache written in place (partial reuse after crash), proxy swap race when opening another clip, no proxy cancel
Final: minor (deferred): keyboard blocked during export (spec says preview stays usable)
Final: minor (deferred): size-advice replies can arrive out of order while dragging
Final: minor (deferred): export thread panic leaves UI on "exporting"
Final: minor (deferred): no keyframes found → lossless copy starts at 0:00
Final: minor (deferred): opening a 1 h+ clip waits for packet scan + thumbnails before showing anything
Final: minor (deferred): fetch-ffmpeg.ps1 downloads "latest" without a pinned version/hash
Final: Ruling: declined-to-judge items (csp null, GPL source offer, AMF/QSV untested, section drag across another, single instance, cover-art streams) stand as-is for v1 — none break a spec promise today — cost if wrong: small follow-ups
```
