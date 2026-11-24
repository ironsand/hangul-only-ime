# 開発者向けメモ

Rust + TSF (Text Services Framework) で実装しています。

## 構成

| パス | 内容 |
|---|---|
| `hangul-core/` | ハングル合成エンジン（OS 非依存・ユニットテスト付き） |
| `hangul-tip/` | TSF テキストサービス DLL（COM in-proc サーバー） |
| `tools/smoke/` | DLL を登録せずに読み込み、COM インターフェースを確認するテスト |
| `tools/e2e/` | インストール済み IME に実際のキー入力を送り、EDIT / RichEdit で結果を検証するテスト |
| `install.ps1` / `uninstall.ps1` | インストール / アンインストール（UAC で管理者に昇格） |

## ビルドとインストール

Rust（MSVC ツールチェーン）が必要です。

```powershell
rustup target add i686-pc-windows-msvc                              # 32bit アプリ用
cargo test -p hangul-core
cargo build --release -p hangul-tip --target x86_64-pc-windows-msvc
cargo build --release -p hangul-tip --target i686-pc-windows-msvc
.\install.ps1
```

## テスト

```powershell
cargo test -p hangul-core                                   # 合成ロジック
cargo run -p smoke -- target\debug\hangul_ime.dll           # COM の基本動作（登録不要）
cargo run -p e2e                                            # インストール済み IME の実入力テスト
```

`e2e` は自分のテスト用ウィンドウにだけ SendInput でキーを送ります（TSF はフォアグラウンドのスレッドにしか
キーを渡さないため）。実行中はキーボード・マウスに触れないでください。前面を失うと即座に中断し、
他のウィンドウには入力しません。

## リリース

`v` で始まるタグ（例：`v0.1.0`）を push すると、GitHub Actions がビルドして Releases に zip を公開します。

## 設計メモ

- **合成中の文字は「暫定文字」(`fInterimChar`) として選択状態にする。** キャレットを合成文字の後ろに置くと、
  韓国語ではアプリが確定扱いにしてコンポジションを終了してしまう。
- **処理しないキーは `OnTestKeyDown` で同期的に確定し、キーは食べない。** 一度「処理した」と返したキーは
  アプリに届かないため。
- **キーボードの open/close コンパートメントを常に open に保つ。** closed だと TSF がキーを IME に渡さない
  （한/영 キーなどで閉じられても再オープンする）。
- すべての COM エントリポイントで panic を捕捉して、ホストアプリへ unwind させない。`DllCanUnloadNow` は
  常に `S_FALSE`（アンロード後のクラッシュを避けるため）。
- 使用中の DLL は上書きできないため、インストーラーは `.old` にリネームしてから差し替える。

## デバッグログ

**デバッグビルド（`--release` なし）でのみ**、環境変数 `HANGUL_IME_LOG=<ファイルパス>` を設定して起動した
プロセスでキー処理のログを書き出します。打鍵内容が残るため、リリースビルドにはこの機能はコンパイルされません
（CI でも確認しています）。

## フォークする場合

自分版を配布するときは、`hangul-tip/src/lib.rs` の `CLSID_HANGUL_IME` と `GUID_PROFILE`（と
`install.ps1` / `uninstall.ps1` の `$Tip`）を新しい GUID に変えてください。同じ GUID の IME が 2 つ
インストールされると、互いの登録を上書きしてしまいます。
