# Worklog

Mac・Windowsで前面のアプリとウィンドウタイトルを定期的に端末内へ保存する、Rust + Tauri 2の作業日誌です。日付別のダッシュボードを見返し、稼働メモをコピーできます。

## 起動

GitHubの[最新リリース](../../releases/latest)から、自分の環境に合うファイルをダウンロードします。

| 環境 | ダウンロードするファイル | 起動方法 |
| --- | --- | --- |
| Apple Silicon Mac（M1以降） | 名前に `aarch64` を含む `.dmg` | DMGを開き、WorklogをApplicationsへ移動 |
| Intel Mac | 名前に `x64` を含む `.dmg` | DMGを開き、WorklogをApplicationsへ移動 |
| Windows 10/11（64-bit） | `x64-setup.exe` | EXEを実行し、スタートメニューからWorklogを起動 |

この段階の配布ファイルにはApple Developer署名・公証とWindowsコード署名を設定していません。初回起動時にOSの確認が表示される場合があります。WindowsでWebView2が入っていない場合は、初回インストール時にインターネット接続が必要です。

macOSでは「システム設定 → プライバシーとセキュリティ → アクセシビリティ」でWorklogを許可してください。取得できなければ「画面収録」も確認し、アプリを再起動してください。Windowsではこの権限設定は不要です。

画面を閉じてもWorklogは通知領域に常駐し、記録を続けます。終了する場合は通知領域のWorklogメニューから「Worklogを終了」を選びます。設定画面では記録間隔、離席判定、除外アプリ、ログイン時の自動起動を変更できます。

## 開発環境から起動

Rustに加え、macOSではXcode Command Line Tools、WindowsではMicrosoft C++ Build ToolsとWebView2が必要です。フロントエンドはHTML/CSS/JavaScriptで、Node.jsやフロントエンドのビルドは不要です。

```sh
cargo run --manifest-path src-tauri/Cargo.toml
```

起動約10秒後に初回取得し、その後は設定した間隔で取得します。最初は別の作業アプリへ切り替えて待ってください。Worklog自身が前面の間は記録をスキップします。

macOSの開発実行では起動元ターミナルへのアクセシビリティ権限が必要な場合があります。権限はアプリから自動変更しません。

ローカル環境向けのインストーラーは次の1コマンドで生成できます。Tauri CLIがなければスクリプトがインストールします。

```sh
./scripts/build-app.sh
```

出力先は `src-tauri/target/release/bundle/` です。macOSではDMG、WindowsのGit BashではNSIS形式のsetup.exeを生成します。

## MVP対象と取得する情報

すべてのアプリを共通の前面ウィンドウ取得で扱い、特定アプリだけに制限していません。macOSではAccessibilityのフォーカス中ウィンドウタイトルを優先し、取得できなければCoreGraphicsのウィンドウタイトルへフォールバックします。フォールバックは同一アプリ内の最前面ウィンドウ候補です。Windowsでは前面ウィンドウのアプリ名とタイトルを取得します。必須対象は以下です。

| アプリ | 取得内容 | プロジェクトの判定 |
| --- | --- | --- |
| VSCode | ウィンドウタイトル（ファイル・ワークスペース） | 標準のタイトル形式からワークスペース候補を抽出 |
| Terminal | 選択中ウィンドウのタイトル | 下記のシェル設定でGitルート名、Git外はディレクトリ名 |
| Firefox | 選択中のタブが公開するウィンドウタイトル | タイトルを条件に分類ルールを追加 |
| Safari | 同上 | 同上 |
| Chrome | 同上 | 同上 |
| Edge | 同上 | 同上 |

CursorにはVSCode同様のタイトル解析、Figmaには書類タイトルの候補抽出があります。Zed・Adobe XD・その他のアプリにも共通取得と分類ルールが使えます。**各アプリ実機での切り替え・権限を含む動作確認はまだ必要です。** 自動テストはタイトル解析・保存・分類・UIを対象とし、全アプリの実動作を保証するものではありません。

ブラウザでは全タブを列挙せず、現在選択されているタブのウィンドウタイトルを保存します。ChatGPTがチャット名をタイトルに反映していればその名前が残ります。タイトルが「ChatGPT」のみならチャット名は不明です。URL、ページ本文、スクリーンショット、キー入力は取得・保存しません。アプリ内部にしかないプロジェクト情報を無条件に取得することはできません。

## Terminalのプロジェクト名

zshの `~/.zshrc` に次を追加してください（この設定は自動では変更しません）。

```sh
source /Users/yutasato/workspace/fetch-focused-window/scripts/terminal-title.zsh
```

新しいターミナルを開くと `worklog:/path/to/project` というウィンドウタイトルを設定します。Git内ではリポジトリルートを使用するため、`src/` に移動しても同じプロジェクトにまとまります。Git外では現在のフォルダ名になります。Terminalのプロファイル設定でタイトルが表示されるようにしてください。他のテーマ・シェル設定・tmuxがタイトルを上書きする場合は調整が必要です。コマンドや別プログラムが一時的にタイトルを変更すると、次のプロンプト表示までプロジェクトが不明になる場合があります。

## プロジェクトの分類

タイムラインの「分類」で、アプリ名・タイトルに含まれる文字列・プロジェクト名を登録します。例：Firefoxのタイトルに `請求管理` を含むものを `顧客A` に分類できます。ブラウザ・エディタ・デザインアプリの別々のルールに同じプロジェクト名を指定すれば、アプリをまたいでまとめられます。

ルールは過去と今後の記録の表示に適用し、元のタイトルを変更しません。複数一致時は新しいルールを優先します。「記録の設定」から削除できます。自動候補は「タイトルから推定」、ルールは「分類ルール」、不明なものは「未分類」と表示します。

## 記録の意味と保存場所

- **設定した間隔ごとの観測記録であり、分単位の正確な実働時間ではありません。** サンプル数を稼働時間として換算しません。短い作業や記録間の切り替えは捕捉できません。
- 設定した時間以上入力がなければ離席として保存します。読書・動画・会議中でも入力がなければ離席になり得ます。
- スリープ・アプリ終了中の記録は作りません。復帰後は次の取得機会から再開し、欠けた時間を埋めません。
- 一時停止状態・分類ルールは再起動後も維持します。
- 保存先：macOSは `~/Library/Application Support/jp.local.worklog/worklog.sqlite3`、Windowsは `%APPDATA%\jp.local.worklog\worklog.sqlite3`。SQLite WALを使います。ネットワークへ送信する実装はありません。
- 稼働メモは選択日の全記録から作成します。検索フィルターには連動しません。コピー前に編集できます。

## CLI・cron

macOSでは既存の単発CLIも使用できます。`cargo run` は一回取得して同じDBに追記します。`memo.txt` は上書きしません。

```sh
cargo build --release
# 独立した保存先で一回だけ記録
WORKLOG_DB=/tmp/worklog.sqlite3 ./target/release/fetch-focused-window
```

cronで実行する場合はバイナリとDBに絶対パスを指定し、macOSの権限も実行元に設定してください。Tauri側の定期取得とcronを同時に使うと二重記録になります。通常はTauriアプリだけを起動してください。

## 開発・検証

```sh
cargo test
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
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

架空の記録を使い、320・375・414・768・1120pxのはみ出し、長い名前、HTMLを含むタイトルの安全な表示、検索、分類保存、停止・再開、メモ生成を確認します。画像は一時ディレクトリの `worklog-ui-test/` に保存します（`UI_SCREENSHOT_DIR` で変更可能）。TauriのAPIはモックのため、実際のウィンドウ取得・権限・DB保存・クリップボードはこのテストの対象外です。

`src/lib.rs` が取得・保存・分類、`src-tauri/src/main.rs` が定期取得とUIのAPI、`ui/` が画面です。`tokens.css` がデザイントークンの原本です。変更時は `cp tokens.css ui/tokens.css` で配布用にも反映してください。

Tauriの構成は[公式ドキュメント](https://v2.tauri.app/develop/)に基づき、静的フロントエンドをバンドルしています。
