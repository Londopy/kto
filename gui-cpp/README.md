# KTO GUI (Windows)

A small Win32 control panel for KTO. It launches `kto.exe`, streams its output
into a log box, lets you pick the interface / target / mode, and has a
"Run KTO at Windows startup" toggle (writes the `HKCU\...\Run` key).

It looks for `kto.exe` next to `kto-gui.exe` first, then falls back to `PATH`.

## Build

Needs the MSVC toolchain (Visual Studio Build Tools) and CMake:

```bat
cmake -S . -B build -A x64
cmake --build build --config Release
:: -> build\Release\kto-gui.exe
```

The release CI builds this automatically and bundles `kto-gui.exe` into the
Windows zip and the installer, so a normal release already includes it.

## Notes

- Pure Win32, no external libraries.
- "Stop" sends a newline to kto's stdin, which is what the CLI watches for to
  shut down cleanly (and run its exit exports).
- The window/taskbar icon comes from `app.rc` (the shared KTO icon).
