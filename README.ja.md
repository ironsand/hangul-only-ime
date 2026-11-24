# Hangul IME

[English](README.md) | 日本語

Windows 用のハングル入力 IME（두벌식 / 2-beolsik 標準配列）です。
英数モードはなく、常にハングル入力です。英語に戻すときは Win+Space で他の入力方式に切り替えます。

## インストール

1. [Releases](../../releases) から `HangulIME-vX.Y.Z.zip` をダウンロードして展開します。
2. 展開したフォルダーの `install.cmd` をダブルクリックし、UAC の確認で「はい」を選びます。
3. Win+Space で「Hangul IME (두벌식)」に切り替えます。

`C:\Program Files\HangulIME\` に DLL を配置して登録し、ユーザーの言語リストの「韓国語」に追加します。
新しい版に更新したときは、起動中のアプリを再起動すると新しい版が使われます。

アンインストールは同じフォルダーの `uninstall.cmd` を実行します。

## 使い方

- 두벌식配列：`dkssudgktpdy` → 안녕하세요、Shift で ㅃㅉㄸㄲㅆㅒㅖ
- Backspace は合成中の字母を 1 つずつ戻します（한 → 하 → ㅎ）
- Space・Enter・記号・矢印・Ctrl/Alt ショートカットは、合成中の文字を確定してからアプリへ渡します

## 既知の制限

- 記号キーの位置は Windows の韓国語キーボードレイアウト（US 配列相当）に従います。JIS キーボードでは
  一部の記号が刻印と異なります（Microsoft の韓国語 IME と同じ挙動）。
- 漢字変換（한자）、세벌식配列、モード表示アイコンはありません。

## 署名と検証について

DLL とスクリプトには**コード署名がありません**。そのため SmartScreen や「発行元を確認できません」の警告が
出ることがあります。配布 zip は GitHub Actions がこのリポジトリのソースからビルドしたものです
（[release.yml](.github/workflows/release.yml)）。

- zip のハッシュはリリースに添付の `.sha256` と比較できます：`Get-FileHash HangulIME-vX.Y.Z.zip`
- 展開したファイルごとのハッシュは `SHA256SUMS.txt` に記録されています。

この IME は**ネットワーク通信やファイルへの書き込みを一切行いません**。

## ソースからビルドする

[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) を参照してください。

## ライセンス

[MIT](LICENSE)
