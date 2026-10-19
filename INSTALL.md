# Castaway Continuum V1.0.0 for Windows

This folder contains the Castaway Continuum application package:

- `CastawayContinuum.exe` — configure and preview the screensaver.
- `CastawayContinuum.scr` — installable Windows screensaver.
- `castaway-continuum.ini` - saved settings, Favorites, and editable scene text.
- `INSTALL.md` - this guide.

## Install

1. Extract this folder to a permanent location. Keep these four files together.
2. Create a `data` folder beside them and copy your legally obtained Johnny
   Castaway data into it. The folder must contain `RESOURCE.MAP` and
   `RESOURCE.001`; keeping the original sound files with those resources is
   recommended.
3. Double-click `CastawayContinuum.exe` to configure display options and preview
   changes.
4. Right-click `CastawayContinuum.scr` and choose **Install**.
5. In Windows Screen Saver Settings, select **Castaway Continuum**.

Windows stores the selected SCR path and normally runs it from this folder. If
you move the entire folder later, right-click the SCR and choose **Install**
again. Do not move only the SCR to `System32`: it needs its sibling EXE and INI.

## Configure

Use `CastawayContinuum.exe` for all settings. In Windows Screen Saver Settings,
the **Settings...** button opens the same configuration application in a window.
Changes save immediately to `castaway-continuum.ini` beside the EXE and SCR.

The actual SCR behaves like a normal screensaver: mouse movement, a click, or a
key press closes it. That is intentional.

## Original data

This public package does **not** include Sierra/Dynamix resource files. Castaway
Continuum needs a legally obtained original data set to display the animation.
Place the original resource files in the `data` subfolder described above, or
beside the EXE/SCR. Do not redistribute those resource files without the
required rights.

## License and attribution

Castaway Continuum is GPL-3.0-or-later software and a fork of Wilson Reborn.
The original Johnny Castaway artwork, animation, music, and sound data remain
copyright Sierra On-Line / Dynamix.
