# Castaway Continuum V1.0.0 Windows Build Guide

This guide builds the Windows desktop application from the complete GPL source in
this repository. It does not provide, download, or redistribute the original
Johnny Castaway resource files.

## Prerequisites

Install the Rust MSVC toolchain and the Visual Studio C++ build tools. The MSVC
toolchain is required for a reliable local Windows build; this checkout's default
GNU toolchain may fail at link time when `dlltool.exe` is not available.

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc --toolchain stable-x86_64-pc-windows-msvc
```

Start a Developer PowerShell for Visual Studio, then run the commands below from
the repository root. The build has been tested with `rust-lld`, which is included
with Rust.

```powershell
$rustLld = 'C:\Users\<you>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
$env:RUSTFLAGS = "-C linker=$rustLld"
rustup run stable-x86_64-pc-windows-msvc cargo build --release -p castaway-continuum --target x86_64-pc-windows-msvc -j 1
```

The resulting application is:

```text
target\x86_64-pc-windows-msvc\release\castaway-continuum.exe
```

## Run with your own data

Place your legally obtained original data beside the executable, in a `data`
subdirectory, or point to it explicitly:

```powershell
.\target\x86_64-pc-windows-msvc\release\castaway-continuum.exe --windowed --data C:\path\to\JohnnyCastawayData
```

The application looks for `RESOURCE.MAP` and `RESOURCE.001`. It can also use the
supported original archive formats described in [INSTALL.md](INSTALL.md).

## Personal-use embedded-data build

An embedded build contains copyrighted Sierra/Dynamix data. Build it only from
your own lawful copy and do not publish the resulting executable.

```powershell
$rustLld = 'C:\Users\<you>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
$env:RUSTFLAGS = "-C linker=$rustLld"
$env:CASTAWAY_CONTINUUM_EMBED_DATA = 'C:\path\to\your\original\data'
rustup run stable-x86_64-pc-windows-msvc cargo build --release -p castaway-continuum --features embed-data --target x86_64-pc-windows-msvc -j 1
```

## Checks

Run source checks before publishing a change:

```powershell
cargo fmt --check
cargo check -p castaway-continuum
```

The Windows icon is optional. A warning about a missing resource compiler
(`rc.exe` or `windres`) does not prevent the executable from running; it only
means Windows will show a generic icon.
