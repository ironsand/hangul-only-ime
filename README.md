# Hangul IME

English | [日本語](README.ja.md)

A Hangul input method for Windows (standard 두벌식 / 2-beolsik layout).
There is no English mode: it always types Hangul. To type English, switch to another input method with Win+Space.

## Installation

1. Download `HangulIME-vX.Y.Z.zip` from [Releases](../../releases) and extract it.
2. Double-click `install.cmd` in the extracted folder and choose "Yes" in the UAC prompt.
3. Switch to "Hangul IME (두벌식)" with Win+Space.

The installer copies and registers the DLLs in `C:\Program Files\HangulIME\` and adds the IME to the Korean entry of
your language list. After updating to a new version, restart running apps to load it.

To uninstall, run `uninstall.cmd` from the same folder.

## Usage

- 2-beolsik layout: `dkssudgktpdy` → 안녕하세요; Shift gives ㅃㅉㄸㄲㅆㅒㅖ
- Backspace removes one jamo at a time from the syllable being composed (한 → 하 → ㅎ)
- Space, Enter, symbols, arrow keys and Ctrl/Alt shortcuts commit the syllable being composed, then reach the app

## Known limitations

- Symbol keys follow the Windows Korean keyboard layout (equivalent to US). On a Japanese (JIS) keyboard some
  symbols differ from the key labels (same as Microsoft's Korean IME).
- No Hanja conversion (한자), no 3-beolsik (세벌식) layouts, no mode indicator icon.

## Signing and verification

The DLLs and scripts are **not code-signed**, so SmartScreen or "unknown publisher" warnings may appear.
Release zips are built from this repository's source by GitHub Actions
([release.yml](.github/workflows/release.yml)).

- Compare the zip's hash with the `.sha256` file attached to the release: `Get-FileHash HangulIME-vX.Y.Z.zip`
- Hashes of the individual files are listed in `SHA256SUMS.txt` inside the zip.

This IME **never communicates over the network and never writes files**.

## Building from source

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## License

[MIT](LICENSE)
