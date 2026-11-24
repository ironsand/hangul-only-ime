# Development notes

Implemented in Rust on top of TSF (Text Services Framework).

## Layout

| Path | Contents |
|---|---|
| `hangul-core/` | Hangul composition engine (OS-independent, with unit tests) |
| `hangul-tip/` | TSF text service DLL (COM in-proc server) |
| `tools/smoke/` | Loads the DLL without registering it and checks its COM interfaces |
| `tools/e2e/` | Sends real keystrokes to the installed IME and checks the result in EDIT / RichEdit controls |
| `installer/` | Inno Setup 6 script for the release installer, plus `langlist.ps1` (adds / removes the IME in the user's language list) |
| `install.ps1` / `uninstall.ps1` | Quick install / uninstall of your local build during development (elevates via UAC) |

## Build and install

Requires Rust with the MSVC toolchain.

```powershell
rustup target add i686-pc-windows-msvc                              # for 32-bit apps
cargo test -p hangul-core
cargo build --release -p hangul-tip --target x86_64-pc-windows-msvc
cargo build --release -p hangul-tip --target i686-pc-windows-msvc
.\install.ps1
```

Do not mix the two ways of installing: `install.ps1` installs into the same folder as the setup .exe but is not
tracked by it. Uninstall one before using the other.

## Tests

```powershell
cargo test -p hangul-core                                   # composition logic
cargo run -p smoke -- target\debug\hangul_ime.dll           # basic COM behavior (no registration needed)
cargo run -p e2e                                            # real input test against the installed IME
```

`e2e` sends keys with SendInput only to its own test windows (TSF delivers keys only to the foreground thread).
Do not touch the keyboard or mouse while it runs. If it loses the foreground it aborts immediately and never types
into other windows.

## Installer

Requires [Inno Setup 6](https://jrsoftware.org/isinfo.php). After building both DLLs:

```powershell
& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" /DAppVersion=0.1.1 installer\HangulIME.iss
# -> dist\HangulIME-Setup-0.1.1.exe
```

(Use `${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe` if Inno Setup was installed for all users.)

## Releases

Pushing a tag starting with `v` (e.g. `v0.1.1`) makes GitHub Actions build the installer and publish it with its
SHA-256 hash on Releases.

## Design notes

- **The syllable being composed is selected as an interim character (`fInterimChar`).** If the caret is placed after
  it instead, apps treat Korean text as committed and terminate the composition.
- **Keys the IME does not handle are committed synchronously in `OnTestKeyDown` and are not eaten.** A key reported
  as handled never reaches the app.
- **The keyboard open/close compartment is kept open.** When it is closed, TSF does not pass keys to the IME
  (it is reopened if something such as the 한/영 key closes it).
- Every COM entry point catches panics so they never unwind into the host app. `DllCanUnloadNow` always returns
  `S_FALSE` (to avoid crashes after unloading).
- A DLL in use cannot be overwritten, so the installers rename it to `.old` before replacing it. On uninstall,
  files that are still loaded are renamed and deleted at the next restart.

## Debug log

**In debug builds only (without `--release`)**, a process started with the environment variable
`HANGUL_IME_LOG=<file path>` writes a log of key handling. Because the log contains keystrokes, this feature is not
compiled into release builds (CI checks this).

## Forking

If you distribute your own build, change `CLSID_HANGUL_IME` and `GUID_PROFILE` in `hangul-tip/src/lib.rs` (and
`$Tip` in `install.ps1` / `uninstall.ps1`) to new GUIDs. Two IMEs installed with the same GUIDs overwrite each
other's registration.
