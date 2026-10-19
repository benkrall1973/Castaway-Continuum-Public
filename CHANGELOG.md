<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Changelog

All notable changes to **Wilson Reborn** are recorded here. The project follows the spirit of
[Keep a Changelog](https://keepachangelog.com/); versions are git tags `vX.Y.Z`.

## [Unreleased]

## [0.3.18] - 2026-10-15

### Changed
- Moved Repeat this scene into the fixed navigation strip and widened the
  evenly aligned touch controls.

## [0.3.17] - 2026-10-15

### Changed
- Moved fixed scene-card navigation into Custom's bottom action area so cards
  with longer descriptions cannot shift the touch controls.

## [0.3.16] - 2026-10-15

### Changed
- Enlarged and reordered scene-card navigation as `|<`, `<=`, `<`, `>`, `=>`,
  `>|`; strengthened the card text and increased description size for touch use.

## [0.3.15] - 2026-10-15

### Changed
- Replaced Custom's scrolling scene list with a touch-friendly scene card
  browser. It presents each reviewed description directly and supports All,
  Normal, Special, Visitors, Common, and Uncommon filters plus first, previous,
  next, last, and five-scene navigation.

## [0.3.14] - 2026-10-15

### Added
- The Custom scene list now shows compact scene numbers and short names, with a
  touch-sized **?** button that opens the reviewed scene description and
  technical identity without changing the selected scene.

## [0.3.13] - 2026-10-15

### Added
- **Custom touch controller:** a shared Windows/Raspberry Pi interface with
  large Environment, Playback, and Scenes tabs; Day/Night, tide, and holiday
  controls; a scrollable verified 63-scene catalog; and Apply controls.
- Ordered playback choices for Normal, Special, **Visitors**, Common, Uncommon,
  and all 63 scenes. Visitors is now an explicit verified `VISITOR.ADS` group.

## [0.3.12] - 2026-10-15

### Fixed
- Kept the **Custom** dialog above the saver during the windowed-to-fullscreen
  switch by dropping the saver out of Windows' topmost level while fullscreen.

## [0.3.11] - 2026-10-15

### Changed
- Renamed **Display > Manager** to **Display > Custom**. The empty Custom
  window is a normal, always-on-top floating dialog with standard title-bar
  minimize, maximize, and close controls.
- Clicking the normal saver display now toggles between framed and fullscreen
  modes. A framed display returns to its prior size and position; the new
  **Full-size window by default** Display option instead restores it maximized.
- Custom stays visible above either display mode until it is minimized or
  closed, and only closes automatically when the saver itself exits.

## [0.3.10] - 2026-10-15

### Added
- **Display Manager shell:** Display > Manager opens a titled, always-on-top
  floating window with standard close and maximize controls. It closes when the
  screensaver exits; its contents will be added separately.

## [0.3.9] - 2026-10-14

### Changed
- Replaced the experimental configuration launcher with native Windows **File**,
  **Display**, **Playback**, and **About** menus for normal playback settings.
- Kept scene, tide, lighting, holiday-override, and positional diagnostics out of
  the normal menus for the dedicated Testing window.

## [0.3.8] - 2026-10-14

### Changed
- Replaced the left-side settings tab rail with Testing, Display, Playback,
  and About items in the Windows-style menu bar.

## [0.3.7] - 2026-10-13

### Added
- **Holiday animation test selector:** force no holiday, Halloween, St. Patrick's Day,
  Christmas, or New Year's props for a selected scene, independent of the current date.

## [0.3.6] - 2026-10-13

### Added
- **Holiday animations checkbox:** preserve the date-based holiday props by default, or suppress
  them for clean scene testing and a non-seasonal display.

## [0.3.5] - 2026-10-13

### Fixed
- **Preview:** no longer replays the optional startup intro before applying a live scene test.

## [0.3.4] - 2026-10-13

### Changed
- **Preview:** applies the complete unsaved settings panel configuration to the running saver,
  including playback speed, scale/filter, dither, sound mute, day/night, tide, transition,
  scene selection, and persistent-window settings. Windowed/fullscreen is applied next launch.

## [0.3.3] - 2026-10-13

### Fixed
- **Shore-wave scheduler:** advances the island shoreline independently of held actor frames,
  while applying only background-pixel deltas so active scene layers are not overwritten.

### Added
- **Tide preference:** authentic/random, prefer high, or prefer low (low tide is selected only
  for scenes authored as low-tide compatible).

## [0.3.2] — 2026-06-26

### Added
- **`SHA256SUMS.txt` in releases**: every release now ships a checksum file covering all artifacts
  (`wilson.scr`/`.exe`, the Linux/macOS tarballs, the `.saver`, and `wilson-web.zip`) so you can
  verify your downloads (`Get-FileHash` on Windows; `sha256sum -c` elsewhere).

### Changed
- **Docs**: the Windows install notes now explain the SmartScreen "unknown publisher" warning (the
  binaries are unsigned for now — *More info → Run anyway*) and that free Authenticode signing via
  the **SignPath Foundation** OSS program is being set up. The README Demo centers the animated GIF
  and drops the redundant static screenshot.

## [0.3.1] — 2026-06-19

### Added
- **Web/WASM bundle in releases**: the release now ships **`wilson-web.zip`** — the bring-your-own
  web build (`index.html` + the wasm/JS + a `HOW-TO-RUN.txt`). No game data is bundled (copyright);
  serve it over HTTP and supply your own `RESOURCE.*` / a `.zip`.
- **Live web page on GitHub Pages** (`pages.yml`): the bring-your-own page is auto-deployed to
  <https://antigerme.github.io/wilson-reborn/> on every push to `main` — visit, bring your own
  data, run it. No game data is hosted (copyright-safe). *(One-time: enable Pages → Source =
  GitHub Actions.)*

### Changed
- **Web page UX (bring-your-own): zero-friction.** Picked data is now **saved on this device by
  default** and the screensaver **auto-starts** — a return visit just opens and runs. Removed the
  **Start**, **Save**, and **dissolve** controls from the page (advanced options stay as URL params,
  e.g. `?dissolve`); a single **🗑 Forget** (in the controls) clears the saved data and returns to
  the picker to load different data. The picker shows in place of the stage (always visible — no
  off-screen panel). Also: CI now runs **once per PR** (no duplicate push+pull_request runs).

### Fixed
- **Web fullscreen via the browser's F11**: pressing **F11** now presents like the page's **⛶**
  button — the canvas fills the screen and the title bar/hints hide (restoring on exit), instead of
  leaving the page's title bar across the top. F11 doesn't engage the Fullscreen API, so it's
  detected via the `(display-mode: fullscreen)` media query. The docs also show how to launch the
  browser already fullscreen (`--kiosk` / `--start-fullscreen`) to open the page fullscreen with no
  click — a true unattended screensaver.
- **Windows preview pane**: the screensaver now fills the little monitor in the Screen Saver
  settings (queries the pane's client size via `GetClientRect`) instead of sitting in the corner
  with black bands.

## [0.3.0] — 2026-06-18

A big polish pass over the **Web/WASM** build and the **Windows screensaver**, plus reverse-engineering
refinements and a real-browser test harness.

### Web / WASM
- **Sound** on by default (Web Audio): per-frame cues with a 🔊/🔇 mute toggle and a **volume** slider.
  Browsers gate audio until the first interaction, so sound now starts on the **first click/keypress/tap**
  and the 🔊 control honestly shows the "waiting" state (it no longer pretends sound is already playing).
- **Fullscreen** ⛶ with a black, letterbox background and a **Screen Wake Lock**; the UI/cursor fade when idle.
- **Matches the desktop defaults**: `scale=fit` + `filter=linear` (was a crisp/nearest look). Plus
  `?scale=fit|stretch|integer` and `?filter=linear|nearest`.
- **Accept a ZIP**: drop a `scrantic-run.zip` *or* `scrantic-installer.zip` (the installer's DCL members are
  decompressed in-wasm), as well as loose `RESOURCE.*` files — with drag-and-drop.
- **Remember data** in the browser (IndexedDB, opt-in) with an explicit **💾 Save** / **Forget**.
- **URL options** mirroring the desktop CLI:
  `?fullscreen & scale & filter & speed & day & dissolve & story & story_secs & daynight & intro & intro_secs
  & mute & volume & seed`.
- Self-contained build option (`embed-data`): bake your own `RESOURCE.*` (+ sounds) into the `.wasm` for a
  no-picker page (personal use; copyright data never committed).

### Desktop & Windows
- **Intro fix**: the intro screen (and first frames) no longer flash by — the winit loop now gates engine
  advance on a per-frame deadline (`FramePacer`) so spurious OS redraws can't race ahead.
- **Intro is configurable**: default **3 s**, tunable with `--intro-secs <1-30>` (config `intro_secs`) /
  `?intro_secs`.
- **Opt-in dissolve leaving the intro**: with `--transition dissolve` / `?dissolve`, the intro dissolves into
  the first scene (default stays a faithful hard cut).
- **Windows `.scr` integration**: no stray console window (GUI subsystem); the **Settings** button opens the
  `config.txt`; the **preview** pane is silent.

### Reverse engineering & faithfulness
- Confirmed by a whole-binary scan that the original's dissolve is **dead code** (gate `[0x1ebf]`: 10 reads,
  0 writes) — the shipped screensaver hard-cuts everywhere, including the intro (KB10 §10.2).
- Architecture docs corrected: tick rate is **16 ms** (`MS_PER_TICK`), not 20 ms.

### Testing
- **End-to-end tests in a real headless Chrome** (Playwright): full render + sound test locally (embedded
  build), and a data-free smoke that runs in **CI** (`web-e2e` job).

### Known issues
- The Windows Settings **preview monitor** renders the screensaver but with black bands (the child window
  doesn't yet fill the pane). Cosmetic; the screensaver itself is unaffected. Fix planned for 0.3.1.

## [0.2.0] — 2026-06-18

First public packaged release: the complete engine on the original data, a live desktop window
(Windows `wilson.scr`/`.exe`, Linux and macOS binaries, a macOS `.saver`), sound, day persistence,
config/options, time controls, the opt-in dissolve transition, the startup intro, and the initial
Web/WASM build. Packaged by `release.yml` on a `v*` tag.

[0.3.2]: https://github.com/antigerme/wilson-reborn/releases/tag/v0.3.2
[0.3.1]: https://github.com/antigerme/wilson-reborn/releases/tag/v0.3.1
[0.3.0]: https://github.com/antigerme/wilson-reborn/releases/tag/v0.3.0
[0.2.0]: https://github.com/antigerme/wilson-reborn/releases/tag/v0.2.0
