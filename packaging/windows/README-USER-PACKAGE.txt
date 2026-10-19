Castaway Continuum for Windows
==============================

This folder is ready to copy to another Windows PC. Keep every file together.
`castaway-continuum.ini` is deliberately stored beside the EXE/SCR so it can be edited
with Notepad and travels with this installation.

Run it
------
1. Double-click CastawayContinuum.exe for a normal windowed/full-screen run.
2. In the Display menu, choose Custom Mode for saved enhancements, or Classic
   Mode for the original scheduling behavior.

Configure it
------------
Use CastawayContinuum.exe as the configuration launcher. Its Display menu opens
Custom Settings, where scene selection, island location, lighting, tide, and
holiday choices are changed. The SCR is intentionally not configurable while
running: mouse movement, a click, or a key press immediately closes it like a
real Windows screensaver. Windows Screen Saver Settings > Settings... opens
this same EXE in windowed configuration mode, so every Display-menu option and
the live preview remain available.

Use it as a Windows screensaver
-------------------------------
1. Extract the ZIP completely and keep the whole folder together. Do not move
  only CastawayContinuum.scr: it needs the sibling CastawayContinuum.exe, castaway-continuum.ini,
   and data\\ folder beside it.
2. Right-click CastawayContinuum.scr and choose Install. Windows registers the
   selected SCR at its current path; it normally runs that exact file and does
   not copy the package into C:\\Windows\\System32.
3. Right-click CastawayContinuum.scr and choose Test to try it immediately.
4. In Windows Screen Saver Settings, select Castaway Continuum and use Settings...
   to open the sibling EXE in live, windowed configuration mode. Closing that
   window returns to Windows Settings; changes have already been saved to the
   castaway-continuum.ini in this package folder.

Keep the installed folder in place
----------------------------------
The SCR starts the EXE from its own folder, and both programs share the INI
there. You may move the entire extracted folder, but afterward choose Install
again so Windows updates its registered SCR path. Do not copy only the SCR to
System32 or another folder: its companion EXE, INI, and resources would no
longer be alongside it.

The application is self-contained for this test package. The original data is
also provided in data\ so the package can be rebuilt or tested as a portable
data-folder installation. Do not separate or redistribute the original data
outside of your authorized personal-use copy.

Notes
-----
- Windows may show a SmartScreen warning because this build is not code-signed.
  Use More info > Run anyway only if you trust the package source.
- `castaway-continuum.ini` contains settings, Favorites, and an editable `[SceneCatalog]`
  section at the bottom. If a scene entry is blank, missing, or malformed, the
  Custom Settings > Scenes tab shows an orange warning and safely uses the
  built-in wording for that entry.
- The Display menu contains Clear favorites... with a confirmation prompt.
  Reset custom values intentionally does not erase Favorites.

Build
-----
Version: 1.0.0
Source branch: main
Source repository: https://github.com/benkrall1973/Castaway-Continuum-Private
