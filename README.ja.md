# Hangul IME

[English](README.md) | 日本語

Windows 用のハングル入力 IME（두벌식 / 2-beolsik 標準配列）です。
英数モードはなく、常にハングル入力です。英語に戻すときは Win+Space で他の入力方式に切り替えます。

## インストール

1. [Releases](../../releases) から `HangulIME-Setup-X.Y.Z.exe` をダウンロードして実行します。
2. Win+Space で「Hangul IME (두벌식)」に切り替えます。

`C:\Program Files\HangulIME\` にインストールし、言語リストの「韓国語」に追加します（韓国語がなければ追加します）。
更新するときは新しい版のインストーラーを実行してください。起動中のアプリは再起動すると新しい版が使われます。

アンインストールは「設定 > アプリ > インストールされているアプリ」の「Hangul IME (두벌식)」から行います。

## 使い方

- 두벌식配列：`dkssudgktpdy` → 안녕하세요、Shift で ㅃㅉㄸㄲㅆㅒㅖ
- Backspace は合成中の字母を 1 つずつ戻します（한 → 하 → ㅎ）
- Space・Enter・記号・矢印・Ctrl/Alt ショートカットは、合成中の文字を確定してからアプリへ渡します

## 既知の制限

- 記号キーの位置は Windows の韓国語キーボードレイアウト（US 配列相当）に従います。JIS キーボードでは
  一部の記号が刻印と異なります（Microsoft の韓国語 IME と同じ挙動）。
- 漢字変換（한자）、세벌식配列、モード表示アイコンはありません。

## Code signing policy（コード署名ポリシー）

リリースには**まだコード署名がありません**。そのため SmartScreen が「発行元不明」の警告を出すことがあります
（「詳細情報」→「実行」で続行できます）。現在、オープンソース向けの無料コード署名プログラムである
SignPath Foundation に申請中です。承認後は、リリースのファイルを次のとおり署名します。

> Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by
> [SignPath Foundation](https://signpath.org/).

- 署名するのは、GitHub Actions がこのリポジトリのソースからビルドしたファイル
  （[release.yml](.github/workflows/release.yml)）だけです。署名リクエストは毎回手動で承認します。
- コミッター・レビュアー：[ironsand](https://github.com/ironsand)
- 承認者：[ironsand](https://github.com/ironsand)

ダウンロードしたファイルは、リリースに添付の `.sha256` とハッシュを比較して確認できます：
`Get-FileHash HangulIME-Setup-X.Y.Z.exe`

### プライバシーポリシー

このプログラムは、利用者またはインストール・運用する人が明示的に求めない限り、他のネットワーク上のシステムに
情報を送信しません。この IME はネットワーク通信やファイルへの書き込みを一切行いません。

## ソースからビルドする

[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)（英語）を参照してください。

## ライセンス

[MIT](LICENSE)
