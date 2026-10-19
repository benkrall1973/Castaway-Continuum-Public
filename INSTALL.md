# Castaway Continuum V1.0.0 for Windows

This folder contains the complete Castaway Continuum application package:

- `CastawayContinuum.exe` — configure and preview the screensaver.
- `CastawayContinuum.scr` — installable Windows screensaver.
- `castaway-continuum.ini` — saved settings, Favorites, and editable scene text.

## Install

1. Extract this folder to a permanent location. Keep all three files together.
2. Double-click `CastawayContinuum.exe` to configure display options and preview
   changes.
3. Right-click `CastawayContinuum.scr` and choose **Install**.
4. In Windows Screen Saver Settings, select **Castaway Continuum**.

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
Place the original resource files beside the EXE/SCR or in a `data` subfolder.
Do not redistribute those resource files without the required rights.

## License and attribution

Castaway Continuum is GPL-3.0-or-later software and a fork of Wilson Reborn.
The original Johnny Castaway artwork, animation, music, and sound data remain
copyright Sierra On-Line / Dynamix.
