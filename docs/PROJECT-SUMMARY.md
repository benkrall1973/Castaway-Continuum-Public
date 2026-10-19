# Castaway Continuum V1.0.0 Project Summary

**Project:** Castaway Continuum - Johnny Castaway

**Status date:** October 15, 2026

## Purpose

This project is a restoration and testing fork of **Wilson Reborn**, released as **Castaway Continuum V1.0.0**. The objective is to preserve the original screensaver's artwork, animation behavior, scene sequencing, timing, and oddities as faithfully as practical, while adding optional testing and usability controls.

The work is not a replacement graphics project. Original Sierra/Dynamix artwork and sound data remain external, user-supplied assets. The engine and the project-specific enhancements are source code built around those original data files.

## Project Goals

- Reproduce the original 63-scene experience faithfully.
- Preserve the 11-day Johnny/Mermaid/Suzy story arc and its clock transitions.
- Correct implementation defects that prevent original content from displaying properly.
- Provide testing controls for selecting and repeatedly viewing individual scenes.
- Keep optional modern display controls separate from authentic/default behavior.
- Retain known original content mistakes unless an explicit optional correction mode is added later.

## License and Asset Boundaries

Castaway Continuum is licensed under **GPL-3.0-or-later**. A distributed executable must be accompanied by the complete corresponding source for that executable, including any local source modifications used to build it.

The original Sierra/Dynamix resource files, artwork, music, and sound effects are not included in the public source repository and must not be redistributed without the appropriate rights. A public release can distribute the GPL source and executable, but should require users to provide their own original game data.

Project documentation should credit:

- Sierra On-Line / Dynamix for the original *Johnny Castaway* artwork, sound, and game content.
- Wilson Reborn as the upstream technical reference and GPL code base.

## Codebase Structure

The current implementation is a Rust workspace.

| Component | Role |
| --- | --- |
| `crates/wilson-dgds` | Decodes the original DGDS resource formats, including resource maps, scripts, images, compression, and animation assets. |
| `crates/wilson-engine` | Runs the animation engine: scene direction, TTM/ADS scripts, island rendering, day/night state, tides, holidays, pathfinding, and scene transitions. |
| `crates/wilson` | Windows desktop/screensaver application, window handling, configuration, settings UI, audio control, rendering presentation, and native screensaver verbs. |
| `crates/wilson-saver` | macOS screensaver integration. |
| `crates/wilson-web` | Browser/WebAssembly build target. |

Useful existing technical references are in [ARCHITECTURE.md](ARCHITECTURE.md), [INSTALL.md](INSTALL.md), and the `docs/reverse-engineering/` directory.

## Faithful Scene Behavior

The engine exposes **63 selectable scenes** for testing:

- **52 non-day/ambient scenes**
- **11 special story scenes**, one for each day of the story arc

The intended special-scene sequence is:

1. The normal island display is interrupted.
2. A clock appears against a black background.
3. The clock hands animate/spin for the authored clock sequence.
4. The clock is removed.
5. The related special story scene plays.
6. The display returns to the normal island activity.

The 11 story scenes are:

| Day | Story scene |
| --- | --- |
| 1 | Mermaid watches Johnny fishing |
| 2 | Johnny imagines himself on the island |
| 3 | Suzy receives Johnny's bottle |
| 4 | Johnny asks Mermaid for a date |
| 5 | Dinner and dance with Mermaid |
| 6 | Johnny draws himself with Suzy |
| 7 | Johnny invites Mermaid to the mainland |
| 8 | Johnny says goodbye and paddles away |
| 9 | Johnny reaches the mainland and meets Suzy |
| 10 | Office dream of the island |
| 11 | Johnny returns by plane |

## Completed Restoration and Testing Work

The following work has been completed and tested in the project testing workspace:

- Corrected special-scene compositing so the clock and story scenes no longer render with a magenta background.
- Corrected the clock presentation so it uses its expected black background rather than appearing over the island.
- Confirmed the 11 clock/story sequences visually and checked their timing against the expected behavior.
- Repaired the shore-wave scheduling regression introduced during tide testing, then returned to a stable wave implementation.
- Added a single-scene testing workflow so any of the 63 scenes can be selected and repeated.
- Added day/night control for scene testing.
- Added island X/Y position override for testing scene edges, horizon placement, and off-screen clipping behavior.
- Added holiday-animation testing override so holiday effects can be viewed on demand rather than waiting for calendar dates.
- Added tide preference testing: authentic/random, prefer high, and prefer low. Scene compatibility must override a preference when a scene requires a specific tide state.
- Improved preview behavior so it can apply the currently selected unsaved testing and playback settings without a full restart or replaying the application intro.
- Added standard controls for windowed mode, sound mute, speed, scaling, filter mode, dedither, and day/night cycle behavior.

## Current Display and Scene Controls

Windows uses a native **Display** menu and two compact floating tools. The user-facing model is:

- **Classic**: original scheduling and no Custom environment overrides.
- **Custom**: saved scene/environment choices apply at startup.
- **Scene Selector > Classic**: restores original scene ordering while retaining active Custom
  lighting, tide, holiday, and saved-island preferences.
- **Scene Sets**: intentionally override scene ordering for browsing/testing.

The Scene Selector supports:

- Selecting one of all 63 scenes and repeating it.
- Searching the scene list.
- Choosing day or night lighting.
- Choosing authentic/random tide behavior, preferred high tide, or preferred low tide.
- Forcing an available holiday animation for testing.
- Applying island X/Y offset overrides.
- Previewing the current unsaved combination of settings.

Classic behaviour remains the normal randomized experience rather than a forced scene or tide.

## Application Settings

The project configuration is stored in `castaway-continuum.ini`. The supported controls currently include:

| Setting | Purpose |
| --- | --- |
| `windowed` | Runs in a movable window instead of fullscreen. |
| `mute` | Disables sound effects. |
| `speed` | Playback speed from 25–400% of original timing. |
| `scale` | Image fill mode: `fit`, `stretch`, `integer`, or `extend`. |
| `filter` | Upscaling filter: `nearest`, `linear`, `xbr`, or `xbrz`. |
| `dedither` | Optionally smooths the original dithered sky and sea. Disabled preserves the authentic look. |
| `daynight` | Uses the original cycle or a real 24-hour clock cycle. |
| `always_on_top` | Keeps a windowed display above other windows. |
| `ignore_mouse_close` | Prevents mouse movement/clicks from closing the display when used as a persistent secondary-screen display. |
| `return_to_primary` | Returns a window to the primary display when enabled and the prior display is unavailable. |
| test scene/day/night/tide/holiday/X/Y values | Temporary scene verification controls. |

## Known Original Content Defects

Several visual problems appear to belong to the original authored assets or scripts rather than to this restoration. They should be preserved for a faithful mode unless a future **Fix original glitches** option is deliberately added.

- Black patch around a fishing rod in one animation.
- A rectangular pirate-ship artifact left in the sea in one sequence.
- A boot appearing without its fishing rod in one sequence.
- Raft work continuing after the raft appears complete.
- The pirate ship can appear to sail through the air when the island is positioned unusually near the horizon; this is tied to original composition and island positioning.
- The plane-flagging animation can show two jumping Johnnys. This is an original scene/script defect, not currently treated as an engine regression.

## Release and Build State

The current Windows release is **V1.0.0**. It includes the accepted native Display and
Playback menus, Classic/Custom profile model, Scene Selector, island controller, and verified
scene catalog.

Windows release packages are ZIP folders. They include `CastawayContinuum.exe`,
`CastawayContinuum.scr`, `castaway-continuum.ini`, a `data` folder, and setup instructions;
they do not include original Sierra/Dynamix resources in public releases.
Sierra/Dynamix resources. Any public GPL binary release must make its exact corresponding source
available to recipients.

## Windows Build Notes

The Windows test executable is built with embedded, user-supplied original data for local testing. The documented working build approach uses the Rust MSVC target and `rust-lld`:

```powershell
$rustLld = 'C:\Users\<you>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
$env:CASTAWAY_CONTINUUM_EMBED_DATA = '<path-to-user-owned-original-data>'
$env:RUSTFLAGS = "-C linker=$rustLld"
cargo build --release -p castaway-continuum --features embed-data --target x86_64-pc-windows-msvc -j 1
```

The executable can also locate original assets externally through the current directory, next to the executable, a `data` directory, `--data <path>`, or `CASTAWAY_CONTINUUM_DATA_DIR`.

The Windows build includes the project's own distributable application icon. A local toolchain
warning about an optional resource compiler should still be investigated, but it does not change
the executable's runtime behaviour.

## Verification Performed

- The 11 special story scenes were visually reviewed after the clock/background correction.
- The 63-scene selector was used to inspect a broad set of normal, day-story, night, tide, and holiday behavior.
- Wave rendering was revalidated after the tide preference regression and rollback/recovery.
- The project builds successfully as a Windows release executable with embedded local test data.
- The Rust workspace passes `cargo check -p castaway-continuum` for the current source state.

## Remaining Work

1. Continue the scene-catalog review and allow user-defined group membership when the catalog is
   ready to be reorganized.
2. Decide whether any original-authored defects should ever receive an optional correction toggle;
   do not silently change faithful behavior.
3. Before a public release, ensure recipients can obtain the matching GPL source and provide
   clean attribution/build instructions alongside the bring-your-own-data package.
4. Keep Windows UI behaviour covered by focused menu-command tests as controls evolve.

## Source Release Contents

This repository contains the Rust workspace, tests, build scripts, GitHub workflow
definitions, reverse-engineering documentation, the project summary, and platform
build instructions. It intentionally excludes original Sierra/Dynamix resource files
and binaries that embed those files. See [BUILD-WINDOWS.md](BUILD-WINDOWS.md) and
[INSTALL.md](INSTALL.md) for build and data-location guidance.

## Practical Rule Going Forward

Treat the latest committed version as the known-good baseline. Make one contained change at a
time, build it, test it against a selected scene, and commit it only after it behaves correctly.
That is slower than randomly shoveling UI experiments into the branch, but it prevents the waves
from mysteriously dying for the fifth goddamn time.
