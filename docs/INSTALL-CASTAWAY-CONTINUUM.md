# Castaway Continuum V1.0.0 installation guide

## Windows package

1. Extract the complete release ZIP to a permanent folder.
2. Keep `CastawayContinuum.exe`, `CastawayContinuum.scr`,
   `castaway-continuum.ini`, and `data\` together.
3. Run `CastawayContinuum.exe` to configure the display and preview changes.
4. Right-click `CastawayContinuum.scr` and choose **Install** to select it in
   Windows Screen Saver Settings.

Windows records the selected SCR path and runs that file. The SCR opens its
sibling EXE for **Settings...**, and both share the INI in the same package
folder. If the whole folder is moved, install the SCR again. Never move only
the SCR to `System32` or another folder.

The real SCR closes immediately on mouse movement, mouse click, or key press.
The EXE is the normal configuration and preview application.

## Original resources

Public distributions must not include Sierra/Dynamix resource data. To run a
bring-your-own-data build, use the complete folder from a lawfully obtained,
already installed or extracted Johnny Castaway copy. Copy **all** files from
that folder into `data\`, place them beside the executable, or select the folder
with `--data <path>`. This includes authorized Internet/archive downloads once
they have been extracted into an installation folder.

At minimum, the folder must contain `RESOURCE.MAP` and its matching
`RESOURCE.001` file. The legacy 16-bit installer executable by itself is not a
replacement for an installed/extracted data folder.

This private repository includes `test-original-data\` solely to reproduce the
owner's embedded local Windows build. Do not redistribute those files unless you
have the necessary rights.

## Build from source

Follow [BUILD-WINDOWS.md](BUILD-WINDOWS.md). For the verified embedded V1.0.0
build, set `CASTAWAY_CONTINUUM_EMBED_DATA` to the resource folder and build the
`castaway-continuum` package with the MSVC target.

## Settings

`castaway-continuum.ini` is created beside the EXE/SCR. It contains display
preferences, Favorites, and the editable `[SceneCatalog]` text. Invalid catalog
entries fall back safely to built-in text and are reported in Custom Settings.
