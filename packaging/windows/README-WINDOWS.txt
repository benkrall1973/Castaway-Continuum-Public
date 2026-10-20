Castaway Continuum - Johnny Castaway for Windows

This package contains the GPL-licensed Castaway Continuum program, forked from Wilson Reborn. It does NOT
contain the original Sierra/Dynamix artwork, animation, music, or sound data.

Quick start
-----------
1. Keep the package folder together: CastawayContinuum.exe, CastawayContinuum.scr,
   castaway-continuum.ini, and
   the data folder must remain beside one another.
2. Double-click CastawayContinuum.exe to run it as a normal program.
3. Right-click CastawayContinuum.scr and choose Install to register it with Windows as a
   screensaver. Windows normally records and runs that exact SCR path; it does
   not copy the package to System32.
4. Keep that folder in place. The SCR opens its sibling EXE for configuration,
   and both share the castaway-continuum.ini beside them. If the whole folder is moved,
   choose Install again to update the Windows registration.

Controls
--------
- Display > Classic: original Johnny Castaway behaviour.
- Display > Custom: enable your saved lighting, tide, island-location, and
  Scene Selector choices.
- Display > Scene Selector: choose Classic scene order or a test Scene Set.
  Choosing Classic preserves Custom environment choices but returns scene
  ordering to the original scheduler.

Data and source
---------------
The original data is copyrighted by Sierra On-Line / Dynamix and is not
redistributed with this project. Create a data folder beside the EXE and SCR,
then copy all files from a lawfully obtained, already installed or extracted
Johnny Castaway folder into it. This includes an authorized Internet/archive
download after it has been extracted into an installation folder. At minimum,
the data folder needs RESOURCE.MAP and RESOURCE.001. Do not use the legacy
16-bit installer executable by itself.

The complete corresponding source code and build instructions are available
from this project's source repository. See BUILD-WINDOWS.md and INSTALL.md in
the source tree for build details.

Windows may show a SmartScreen warning because this program is not code-signed.
Verify the supplied SHA256SUMS.txt before running a downloaded release.
