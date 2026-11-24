# Hangul IME

English | [日本語](README.ja.md)

A Hangul input method for Windows (standard 두벌식 / 2-beolsik layout).
There is no English mode: it always types Hangul. To type English, switch to another input method with Win+Space.

## Installation

1. Download `HangulIME-Setup-X.Y.Z.exe` from [Releases](../../releases) and run it.
2. Switch to "Hangul IME (두벌식)" with Win+Space.

The installer installs the IME in `C:\Program Files\HangulIME\` and adds it to Korean in your language list
(Korean is added if it is not there yet). To update, run the installer of the new version; restart running apps to
load it.

To uninstall, go to Settings > Apps > Installed apps > "Hangul IME (두벌식)".

## Usage

- 2-beolsik layout: `dkssudgktpdy` → 안녕하세요; Shift gives ㅃㅉㄸㄲㅆㅒㅖ
- Backspace removes one jamo at a time from the syllable being composed (한 → 하 → ㅎ)
- Space, Enter, symbols, arrow keys and Ctrl/Alt shortcuts commit the syllable being composed, then reach the app

## Known limitations

- Symbol keys follow the Windows Korean keyboard layout (equivalent to US). On a Japanese (JIS) keyboard some
  symbols differ from the key labels (same as Microsoft's Korean IME).
- No Hanja conversion (한자), no 3-beolsik (세벌식) layouts, no mode indicator icon.

## Signing and verification

The installer and the IME are **not code-signed**, so Windows SmartScreen may warn about an unknown publisher
(click "More info" → "Run anyway"). Release installers are built from this repository's source by GitHub Actions
([release.yml](.github/workflows/release.yml)). To check a download, compare its hash with the `.sha256` file
attached to the release: `Get-FileHash HangulIME-Setup-X.Y.Z.exe`

This IME **never communicates over the network and never writes files**.

## Building from source

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## License

[MIT](LICENSE)
