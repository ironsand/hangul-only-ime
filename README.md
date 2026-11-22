# Hangul IME

Windows 用のハングル入力 IME（두벌식 / 2-beolsik 標準配列）。Rust + TSF (Text Services Framework)。
英数モードはなく、常にハングル入力です。英語に戻すときは Win+Space で他の入力方式に切り替えます。

## 構成

| パス | 内容 |
|---|---|
| `hangul-core/` | ハングル合成エンジン（OS 非依存・ユニットテスト付き） |
| `hangul-tip/` | TSF テキストサービス DLL（COM in-proc サーバー） |
| `tools/smoke/` | DLL を登録せずに読み込み、COM インターフェースを確認するテスト |
| `tools/e2e/` | インストール済み IME に実際のキー入力を送り、EDIT / RichEdit で結果を検証するテスト |
| `install.ps1` / `uninstall.ps1` | インストール / アンインストール（UAC で管理者に昇格） |

## ビルドとインストール

```powershell
cargo test -p hangul-core
cargo build --release -p hangul-tip --target x86_64-pc-windows-msvc
cargo build --release -p hangul-tip --target i686-pc-windows-msvc   # 32bit アプリ用
.\install.ps1
```

`C:\Program Files\HangulIME\{x64,x86}\` に配置・登録し、ユーザーの言語リストの「韓国語」に追加します。
使用中の DLL は `.old` にリネームして差し替えるので、更新後は対象アプリを再起動すると新しい版になります。

## テスト

```powershell
cargo test -p hangul-core                                   # 合成ロジック
cargo run -p smoke -- target\debug\hangul_ime.dll           # COM の基本動作（登録不要）
cargo run -p e2e                                            # インストール済み IME の実入力テスト
```

`e2e` は自分のテスト用ウィンドウにだけ SendInput でキーを送ります（TSF はフォアグラウンドのスレッドにしか
キーを渡さないため）。実行中はキーボード・マウスに触れないでください。前面を失うと即座に中断し、
他のウィンドウには入力しません。

## 使い方

- 두벌식配列：`dkssudgktpdy` → 안녕하세요、Shift で ㅃㅉㄸㄲㅆㅒㅖ
- Backspace は合成中の字母を 1 つずつ戻す（한 → 하 → ㅎ）
- Space・Enter・記号・矢印・Ctrl/Alt ショートカットは、合成中の文字を確定してからアプリへ渡す

## 設計メモ

- **合成中の文字は「暫定文字」(`fInterimChar`) として選択状態にする。** キャレットを合成文字の後ろに置くと、
  韓国語ではアプリが確定扱いにしてコンポジションを終了してしまう。
- **処理しないキーは `OnTestKeyDown` で同期的に確定し、キーは食べない。** 一度「処理した」と返したキーは
  アプリに届かないため。
- **キーボードの open/close コンパートメントを常に open に保つ。** closed だと TSF がキーを IME に渡さない
  （한/영 キーなどで閉じられても再オープンする）。
- すべての COM エントリポイントで panic を捕捉して、ホストアプリへ unwind させない。`DllCanUnloadNow` は
  常に `S_FALSE`（アンロード後のクラッシュを避けるため）。

## デバッグログ

環境変数 `HANGUL_IME_LOG=<ファイルパス>` を設定して起動したプロセスでだけ、キー処理のログを書き出します。

## 既知の制限

- 記号キーの位置は Windows の韓国語キーボードレイアウト（US 配列相当）に従います。JIS キーボードでは
  一部の記号が刻印と異なります（Microsoft の韓国語 IME と同じ挙動）。
- 漢字変換（한자）、세벌식配列、モード表示アイコンはありません。
