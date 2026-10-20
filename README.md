<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Castaway Continuum

Castaway Continuum is a Windows-focused enhancement of the 1992 Johnny Castaway
screensaver. It keeps the original animation and data format intact while making
the experience easier to configure, inspect, and enjoy on modern displays.

It is based on [Wilson Reborn by Antigerme](https://github.com/antigerme/wilson-reborn).
That repository is the upstream project; this repository is the complete source
for Castaway Continuum V1.0.0.

## What changed

Castaway Continuum concentrates on faithful playback first, then adds controls
that the original screensaver never had.

### Animation and event improvements

- The independent shore-wave scheduler keeps the island and shoreline moving
  naturally even while an actor animation holds on a frame.
- The eleven special-story days are catalogued and played in chronological order,
  from Day 1 through Day 11, when the Special set is selected.
- Holiday props can follow the real calendar or be deliberately selected for
  viewing: Halloween, St. Patrick's Day, Christmas, or New Year.
- Low tide is only used by scenes that support it; unsupported scenes clearly
  fall back to high tide instead of producing broken artwork.
- Classic Mode remains available whenever the original randomized behavior is
  wanted with no custom scene-order rules.

### Display and playback controls

The native Windows **Display** menu provides practical controls for lighting,
tide, holidays, scaling, pixel filtering, smooth water and sky dithering, window
behavior, and display placement. A Custom Mode/Classic Mode switch keeps saved
enhancements separate from the original-style experience.

Optional CRT presentation controls include scanlines, gentle-to-strong screen
curvature, phosphor masks, bloom, color profiles, and horizontal softness. They
are visual filters only: the source animation data is not altered.

> **Performance:** CRT effects such as curvature, bloom, phosphor masks, and
> horizontal softness increase rendering load. On lower-power PCs, use scanlines
> only or disable CRT effects for smoother playback.

### Custom Settings and Scene Selector

The Custom Settings window puts the controls that need a visual reference in one
place:

- **Scenes:** select a set, jump to First, -5 Scenes, Previous, Next, +5 Scenes,
  or Last, and repeat a selected scene.
- **Special:** browse the verified Day 1-11 story sequence in order.
- **Favorites:** mark scenes as favorites and play only that personal set; the
  Display menu can clear it after confirmation.
- **Island Location:** move the island with five-pixel steps and save a preferred
  position for Custom Mode.
- **Lighting, Tide, and Holiday:** apply choices immediately, with no separate
  Apply button and no buried state waiting to surprise somebody later.

The selector displays a short scene name, a plain-language description, technical
animation filename, and the low-tide availability status. An optional on-screen
overlay can show the current scene title and description in any display corner.

Scene descriptions, short names, favorites, and settings are stored in
`castaway-continuum.ini` beside the executable, so a user can edit the text and
keep their preferences with the application.

## Install and run

See [INSTALL.md](INSTALL.md) for the Windows package and screensaver setup.
Run `CastawayContinuum.exe` to configure and preview. Right-click
`CastawayContinuum.scr` and choose **Install** to register the screensaver.

## Original data is required

This project does **not** include Johnny Castaway artwork, animations, music, or
sound effects. Supply a legally obtained original data set containing
`RESOURCE.MAP` and `RESOURCE.001`, normally in a `data` folder beside the EXE
and SCR. See [docs/BUILD-WINDOWS.md](docs/BUILD-WINDOWS.md) for build details.

## Build from source

The Windows application uses Rust and the MSVC toolchain. From a Developer
PowerShell for Visual Studio:

```powershell
$rustLld = 'C:\Users\<you>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
$env:RUSTFLAGS = "-C linker=$rustLld"
rustup run stable-x86_64-pc-windows-msvc cargo build --release -p castaway-continuum --target x86_64-pc-windows-msvc -j 1
```

The built executable is placed at
`target\x86_64-pc-windows-msvc\release\castaway-continuum.exe`.

## License and credits

Castaway Continuum is GPL-3.0-or-later. The original Johnny Castaway artwork,
animation, music, and sound data remain the property of Sierra On-Line / Dynamix
and are not redistributed here. Upstream source and project credit belong to
[Antigerme's Wilson Reborn](https://github.com/antigerme/wilson-reborn).
