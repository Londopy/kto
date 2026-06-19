# Releasing KTO

Releases are fully automated by [`.github/workflows/release.yml`](../.github/workflows/release.yml).

## Cutting a release

1. Bump `version` in `Cargo.toml` and add a `CHANGELOG.md` section.
2. Commit, then tag and push:

   ```bash
   git tag v3.0.0
   git push origin v3.0.0
   ```

3. The `release` workflow runs and, on success, publishes a GitHub release with:
   - `kto-<ver>-x86_64-unknown-linux-gnu.tar.gz`
   - `kto-<ver>-aarch64-unknown-linux-gnu.tar.gz`
   - `kto-<ver>-x86_64-apple-darwin.tar.gz`
   - `kto-<ver>-aarch64-apple-darwin.tar.gz`
   - `kto-<ver>-x86_64-pc-windows-msvc.zip`  (contains `kto.exe` + `kto-gui.exe`)
   - `kto-<ver>-setup-x64.exe`  (Inno Setup installer)
   - `kto-<ver>-SHA256SUMS.txt`  (combined SHA-256 checksums for every asset)

`workflow_dispatch` is also available for dry runs (it builds but does not
publish, since the publish job is gated on a tag).

## Verifying checksums

```bash
# Linux/macOS
sha256sum -c kto-3.0.0-SHA256SUMS.txt      # or: shasum -a 256 -c ...

# Windows (PowerShell)
Get-FileHash .\kto-3.0.0-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

The `SHA256SUMS.txt` lines are in the standard `<hash>  <filename>` format, so
`sha256sum -c` works directly when the assets sit next to it.

## Windows installer (Inno Setup)

[`installer/kto.iss`](../installer/kto.iss) builds an x64 installer that drops
`kto.exe` into `Program Files\KTO`, adds Start-menu shortcuts, and optionally
appends the install dir to the system `PATH` (with clean removal on uninstall).

Build it locally with [Inno Setup 6](https://jrsoftware.org/isdl.php):

```bat
mkdir dist
copy target\release\kto.exe dist\
copy README.md dist\ & copy LICENSE dist\ & copy CHANGELOG.md dist\
iscc /DMyAppVersion=3.0.0 /DSourceDir=dist installer\kto.iss
:: output: installer\installer-out\kto-3.0.0-setup-x64.exe
```

## Scoop

[`scoop/kto.json`](../scoop/kto.json) is a Scoop manifest. Two ways to ship it:

- **Quick (single manifest):** users install straight from the raw URL:

  ```powershell
  scoop install https://raw.githubusercontent.com/Londopy/kto/main/scoop/kto.json
  ```

- **Bucket (recommended):** put the manifest in a `bucket/` folder of a repo
  (e.g. `Londopy/scoop-bucket`), then:

  ```powershell
  scoop bucket add londopy https://github.com/Londopy/scoop-bucket
  scoop install kto
  ```

The manifest uses `"checkver": "github"` and an `autoupdate` block whose `hash`
is read from the release's `SHA256SUMS.txt`, so `scoop update` and the
`excavator`/`bin/auto-pr` tooling can bump `version`, `url`, and `hash`
automatically after each release. The committed `hash` placeholder
(`0000…`) is replaced by autoupdate on the first bump - or set it by hand:

```powershell
# from a scoop-bucket checkout
.\bin\checkver.ps1 kto -Update
```
