# Worklog

Mac・Windowsで前面のアプリとウィンドウタイトルを定期的に端末内へ保存する、Rust + Tauri 2の作業日誌です。日付別のダッシュボードを見返し、稼働メモをコピーできます。

## 起動

GitHubの[最新リリース](../../releases/latest)から、自分の環境に合うファイルをダウンロードします。

| 環境 | ダウンロードするファイル | 起動方法 |
| --- | --- | --- |
| Apple Silicon Mac（M1以降） | 名前に `aarch64` を含む `.dmg` | DMGを開き、WorklogをApplicationsへ移動 |
| Intel Mac | 名前に `x64` を含む `.dmg` | DMGを開き、WorklogをApplicationsへ移動 |
| Windows 10/11（64-bit） | `x64-setup.exe` | EXEを実行し、スタートメニューからWorklogを起動 |

現在のmacOS版は、Apple Developer ID署名とAppleのnotarizationを行っていません。初回起動時に「Appleは、“Worklog”にMacに損害を与えたり、プライバシーを侵害する可能性のあるマルウェアが含まれていないことを検証できませんでした」と表示される場合は、Worklogを右クリックして「開く」を選び、それでも止まる場合は「システム設定 → プライバシーとセキュリティ」でWorklogの起動を許可してください。

許可しても同じダイアログが再表示される場合は、ダウンロード由来のquarantine属性がアプリ内に残っている可能性があります。信頼できる自分のビルドだけ、次のコマンドで解除できます。

```sh
xattr -dr com.apple.quarantine /Applications/Worklog.app
```

WindowsでWebView2が入っていない場合は、初回インストール時にインターネット接続が必要です。

macOSでは初回起動時に、インストールしたWorklog自身がアクセシビリティ許可を要求します。「システム設定 → プライバシーとセキュリティ → アクセシビリティ」でWorklogを有効にして、アプリを再起動してください。設定画面の「データと権限」には、許可対象になっている実行ファイルのパスと現在の状態も表示されます。Windowsではこの権限設定は不要です。

画面を閉じてもWorklogは通知領域に常駐し、記録を続けます。終了する場合は通知領域のWorklogメニューから「Worklogを終了」を選びます。設定画面では記録間隔、離席判定、除外アプリ、ログイン時の自動起動を変更できます。

## 開発環境から起動

Rustに加え、macOSではXcode Command Line Tools、WindowsではMicrosoft C++ Build ToolsとWebView2が必要です。フロントエンドはHTML/CSS/JavaScriptで、Node.jsやフロントエンドのビルドは不要です。

```sh
cargo run --manifest-path src-tauri/Cargo.toml
```

起動約10秒後に初回取得し、その後は設定した間隔で取得します。最初は別の作業アプリへ切り替えて待ってください。Worklog自身が前面の間は記録をスキップします。

macOSの開発実行では起動元Terminalと、DMGから`/Applications`へインストールしたWorklogは別の実行主体として扱われます。配布版の動作確認はインストール済みWorklogを起動し、そのアプリから表示される許可要求を使ってください。設定画面に`/Applications/Worklog.app/Contents/MacOS/worklog`と表示されていれば、インストール版を操作しています。

開発版ではログイン時の自動起動を設定できません。リリース版は起動時に既存の自動起動設定を現在のインストール先へ更新するため、ワークスペース内の古いビルド成果物が次回ログイン時に起動することを防ぎます。

ローカル環境向けのインストーラーは次の1コマンドで生成できます。Tauri CLIがなければスクリプトがインストールします。

```sh
./scripts/build-app.sh
```

出力先は `src-tauri/target/release/bundle/` です。macOSではDMG、WindowsのGit BashではNSIS形式のsetup.exeを生成します。

## MVP対象と取得する情報

すべてのアプリを共通の前面ウィンドウ取得で扱い、特定アプリだけに制限していません。macOSではAccessibilityのフォーカス中ウィンドウタイトルを優先し、取得できなければCoreGraphicsのウィンドウタイトルへフォールバックします。フォールバックは同一アプリ内の最前面ウィンドウ候補です。Windowsでは前面ウィンドウのアプリ名とタイトルを取得します。必須対象は以下です。

| アプリ | 取得内容 | プロジェクトの判定 |
| --- | --- | --- |
| VS Code系（Code / Visual Studio Code / Cursor / VSCodium / Windsurfなど） | ウィンドウタイトル（ファイル・ワークスペース） | `file — workspace` や `file - workspace - Visual Studio Code` からワークスペース候補を抽出 |
| Zed | ウィンドウタイトル（プロジェクト・ファイル） | `project — path/to/file` から先頭のプロジェクト候補を抽出 |
| JetBrains IDE / Xcode | ウィンドウタイトル（プロジェクト・ファイル） | IDEごとの既知形式からプロジェクト候補を抽出 |
| Obsidian / Figma / Sublime Text / Office / iWork / Sketch / Adobe系 | ウィンドウタイトル（書類・Vaultなど） | Obsidianなどは既知形式から抽出。Office / iWork / Sketch / Adobe XDはウィンドウタイトルを主に書類名として扱う |
| Terminal / Windows Terminal | 選択中ウィンドウのタイトル | 下記のシェル設定でGitルート名、Git外はディレクトリ名 |
| Firefox | 選択中のタブが公開するウィンドウタイトル | タイトルを条件に分類ルールを追加 |
| Safari | 同上 | 同上 |
| Chrome | 同上 | 同上 |
| Edge | 同上 | 同上 |

アプリ名は部分一致ではなく、大文字小文字を無視した完全一致または製品名にエディション名・年などが続く形式だけで判定します。CursorはVS Code系、Figmaは書類タイトル、Zedは先頭プロジェクト、Adobe XDは書類タイトルの解析があります。形式に合わないタイトル、ようこそ画面、1要素だけでファイル名やダイアログ名と区別できないタイトルはプロジェクト不明として扱います。ただしOffice / iWork / Sketch / Adobe XDは、ウィンドウタイトルからアプリ名・Untitled・拡張子などを除いた名前を主に書類名として扱うため、環境設定などのダイアログが前面にあるとその名前がプロジェクト候補になる場合があります。**各アプリ実機での切り替え・権限を含む動作確認はまだ必要です。** 自動テストはタイトル解析・保存・分類・UIを対象とし、全アプリの実動作を保証するものではありません。

ブラウザでは全タブを列挙せず、現在選択されているタブのウィンドウタイトルを保存します。ChatGPTがチャット名をタイトルに反映していればその名前が残ります。タイトルが「ChatGPT」のみならチャット名は不明です。URL、ページ本文、スクリーンショット、キー入力は取得・保存しません。アプリ内部にしかないプロジェクト情報を無条件に取得することはできません。

## Terminalのプロジェクト名

アプリの「設定 → ターミナルのプロジェクト名」に表示されるスニペットを、zshの `~/.zshrc` に追加してください（この設定は自動では変更しません）。ソースコードをcloneして使う場合は、次のようにclone先の絶対パスを指定する方法もあります。

```sh
source /path/to/cloned/worklog/scripts/terminal-title.zsh
```

新しいターミナルを開くと `worklog:/path/to/project` というウィンドウタイトルを設定します。Git内ではリポジトリルートを使用するため、`src/` に移動しても同じプロジェクトにまとまります。Git外では現在のフォルダ名になります。Terminalのプロファイル設定でタイトルが表示されるようにしてください。他のテーマ・シェル設定・tmuxがタイトルを上書きする場合は調整が必要です。コマンドや別プログラムが一時的にタイトルを変更すると、次のプロンプト表示までプロジェクトが不明になる場合があります。

Windows Terminal / PowerShellでは、PowerShellプロファイルに次を追加できます。既存の `prompt` 関数はラップして呼び出すため、プロンプト表示はそのまま使えます。Windows実機ではまだ動作確認していません。

```powershell
. "C:\path\to\cloned\worklog\scripts\terminal-title.ps1"
```

## プロジェクトの分類

タイムラインの「分類」で、記録から取得したアプリ名・タイトルに含まれる文字列・プロジェクト名を登録します。アプリ名とタイトルの文字列は記録から決まり、ダイアログ内では編集できません。例：Firefoxのタイトルに `請求管理` を含むものを `顧客A` に分類できます。ブラウザ・エディタ・デザインアプリの別々のルールに同じプロジェクト名を指定すれば、アプリをまたいでまとめられます。

分類ダイアログでは、タイトルを ` — ` や ` - ` などの区切りで分けた候補とタイトル全体から、分類に使う文字列を選べます。保存前に、表示中の日の記録に何件一致するかと例を確認できます。

同じ条件のルールがすでにある場合や、ほかのルールと一致する記録が重なる場合は、保存前に警告を表示します。タイトルを取得できなかった記録は、部分一致の条件を作れないため分類できません。

ルールは過去と今後の記録の表示に適用し、元のタイトルを変更しません。複数一致時は新しいルールを優先します。分類ルールは設定画面、またはタイムラインの「分類解除」から削除できます。自動候補は「タイトルから推定」、ルールは「分類ルール」、不明なものは「未分類」と表示します。

## 記録の意味と保存場所

- **設定した間隔ごとの観測記録であり、分単位の正確な実働時間ではありません。** 画面の分数は必ず推定として表示し、各記録に保存された記録間隔から計算します。記録間隔が保存されていない古い記録（v0.1.x）は、前後の記録との間隔から推定します。
- タイムラインでは、アプリ名・タイトル・状態が同じ記録が続き、間隔が前回記録の1.5倍以内なら1つのまとまりとして表示します。まとまりの終了時刻は、最後の記録時刻＋記録間隔、次の別記録、現在時刻のうち一番早い時刻です。
- レポートでは、選択日のまとまりをプロジェクト別に集計します。離席は作業時間には含めず、離席時間として分けて表示します。
- 短い作業や記録間の切り替えは捕捉できません。
- 設定した時間以上入力がなければ離席として保存します。読書・動画・会議中でも入力がなければ離席になり得ます。
- スリープ・アプリ終了中の記録は作りません。復帰後は次の取得機会から再開し、欠けた時間を埋めません。
- 一時停止状態・分類ルールは再起動後も維持します。
- 保存先：macOSは `~/Library/Application Support/jp.local.worklog/worklog.sqlite3`、Windowsは `%APPDATA%\jp.local.worklog\worklog.sqlite3`。SQLite WALを使います。ネットワークへ送信する実装はありません。
- 稼働メモは選択日のまとまりから古い順に作成します。検索フィルターには連動しません。コピー前に編集できます。

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

架空の記録を使い、720・768・1000・1120pxのはみ出し、長い名前、HTMLを含むタイトルの安全な表示、検索、分類保存、停止・再開、メモ生成を確認します。Worklogのウィンドウ最小幅は720pxです。画像は一時ディレクトリの `worklog-ui-test/` に保存します（`UI_SCREENSHOT_DIR` で変更可能）。TauriのAPIはモックのため、実際のウィンドウ取得・権限・DB保存・クリップボードはこのテストの対象外です。

`src/lib.rs` が取得・保存・分類、`src-tauri/src/main.rs` が定期取得とUIのAPI、`ui/` が画面です。`tokens.css` がデザイントークンの原本です。変更時は `cp tokens.css ui/tokens.css` で配布用にも反映してください。

Tauriの構成は[公式ドキュメント](https://v2.tauri.app/develop/)に基づき、静的フロントエンドをバンドルしています。

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

開発中にこのMacだけでアクセシビリティ許可を安定させたい場合は、ログインキーチェーンに `Worklog Local Code Signing` というコード署名IDを作成して使えます。`scripts/build-app.sh` はこの署名IDが存在する場合、macOSビルド時に自動で `APPLE_SIGNING_IDENTITY=Worklog Local Code Signing` を使います。これは他のMacへ配布するための署名ではありません。
