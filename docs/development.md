# 開発ガイド

[README](../README.md)の「開発」より詳しい情報です。コードの構成は[ARCHITECTURE.md](../ARCHITECTURE.md)、画面のデザインルールは[design.md](../design.md)を見てください。

## 開発環境から起動

Rustに加え、macOSではXcode Command Line Tools、WindowsではMicrosoft C++ Build ToolsとWebView2が必要です。フロントエンドはHTML/CSS/JavaScriptで、Node.jsやフロントエンドのビルドは不要です。

```sh
cargo run -p worklog
```

起動約10秒後に初回取得し、その後は設定した間隔で取得します。最初は別の作業アプリへ切り替えて待ってください。Worklog自身が前面の間は記録をスキップします。

macOSの開発実行では起動元Terminalと、DMGから`/Applications`へインストールしたWorklogは別の実行主体として扱われます。配布版の動作確認はインストール済みWorklogを起動し、そのアプリから表示される許可要求を使ってください。設定画面に`/Applications/Worklog.app/Contents/MacOS/worklog`と表示されていれば、インストール版を操作しています。

開発版ではログイン時の自動起動を設定できません。リリース版は起動時に既存の自動起動設定を現在のインストール先へ更新するため、ワークスペース内の古いビルド成果物が次回ログイン時に起動することを防ぎます。

## インストーラーの作成

ローカル環境向けのインストーラーは次の1コマンドで生成できます。Tauri CLIがなければスクリプトがインストールします。

```sh
./scripts/build-app.sh
```

出力先は `target/release/bundle/` です。macOSではDMG、WindowsのGit BashではNSIS形式のsetup.exeを生成します。

`scripts/build-app.sh` は、ログインキーチェーンに `Worklog Local Code Signing` という名前のコード署名IDがあれば、macOSビルド時に `APPLE_SIGNING_IDENTITY=Worklog Local Code Signing` を使います。同じ環境でローカルビルドし直したときにアクセシビリティ許可が外れにくくなりますが、配布用の署名ではありません。

## CLIデバッグ

`worklog-core` crate の `cargo run -p worklog-core` は、前面ウィンドウを1回だけ取得してDBに追記するデバッグ用のCLIです。通常はTauriアプリだけを起動してください。アプリと同時に使うと二重記録になります。

```sh
cargo run -p worklog-core
WORKLOG_DB=/tmp/worklog.sqlite3 cargo run -p worklog-core
```

`WORKLOG_DB` を指定すると保存先を変えられます。未指定時はどのOSでも `$HOME/Library/Application Support/jp.local.worklog/worklog.sqlite3` を使います。`HOME` がない場合は「HOMEがありません。WORKLOG_DBを指定してください。」で終了します。

## 検証

引数なしの `cargo test` はコアのテストだけを実行します。

```sh
cargo test --locked
cargo check --locked --workspace
cargo fmt --all --check
node --check ui/dashboard.js
zsh -n scripts/terminal-title.zsh
```

UIのモック検証にはNode.jsとPlaywrightを使います。依存はプロジェクト外にインストールできます。

```sh
npm install --prefix /tmp/worklog-ui-test playwright
/tmp/worklog-ui-test/node_modules/.bin/playwright install chromium
PLAYWRIGHT_MODULE=/tmp/worklog-ui-test/node_modules/playwright node scripts/ui-smoke.cjs
```

既存のChromeを使う場合はブラウザインストールを省き、`CHROME_PATH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"` を実行時に指定できます。通常の依存にPlaywrightがある場合は `node scripts/ui-smoke.cjs` だけで実行できます。

架空の記録を使い、720・768・1000・1120pxのはみ出し、長い名前、HTMLを含むタイトルの安全な表示、検索、分類保存、停止・再開を確認します。Worklogのウィンドウ最小幅は720pxです。画像は一時ディレクトリの `worklog-ui-test/` に保存します（`UI_SCREENSHOT_DIR` で変更可能）。TauriのAPIはモックのため、実際のウィンドウ取得・権限・DB保存はこのテストの対象外です。

## 配布署名

GitHub ActionsでmacOSの公証済みDMGを作るには、リポジトリのActions secretsに次を設定してください。

| Secret | 内容 |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application証明書を書き出した `.p12` のbase64 |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` の書き出し時に設定したパスワード |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: ...` の署名ID |
| `APPLE_API_KEY` | App Store Connect API Key ID |
| `APPLE_API_ISSUER` | App Store Connect Issuer ID |
| `APPLE_API_KEY_BASE64` | `AuthKey_XXXXXXXXXX.p8` のbase64 |

`APPLE_API_KEY_BASE64` は次のように作成できます。

```sh
base64 -i AuthKey_XXXXXXXXXX.p8 | tr -d '\n' | pbcopy
```

macOS向けにDeveloper ID証明書で署名する場合はnotarizationが必要です。6つのSecretsがすべて未設定の場合、CIは公証なしのDMGを作って公開します。一部だけ設定されている場合は、設定漏れとしてCIを失敗させます。
