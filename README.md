# Worklog

前面のアプリ名とウィンドウタイトルを定期的に記録して、日ごとの作業を振り返るためのMac・Windows用アプリです。記録はこの端末の中だけに保存し、ネットワークへは送信しません。

## インストール

[最新リリース](../../releases/latest)から、環境に合うファイルをダウンロードします。

| 環境 | ファイル | インストール方法 |
| --- | --- | --- |
| Apple Silicon Mac（M1以降） | 名前に `aarch64` を含む `.dmg` | DMGを開き、WorklogをApplicationsへ移動 |
| Intel Mac | 名前に `x64` を含む `.dmg` | DMGを開き、WorklogをApplicationsへ移動 |
| Windows 10/11（64-bit） | `x64-setup.exe` | EXEを実行 |

macOSでは、最初に次の2つが必要です。

1. **起動の許可**：公証をしていないため、「検証できませんでした」と表示されることがあります。Worklogを右クリックして「開く」を選んでください。
2. **アクセシビリティの許可**：「システム設定 → プライバシーとセキュリティ → アクセシビリティ」でWorklogを有効にし、Worklogを再起動してください。ウィンドウタイトルの取得に使います。

うまくいかないときは[初回起動でつまずいたとき](docs/usage.md#初回起動でつまずいたとき)を見てください。

## 使い方

- 画面を閉じても通知領域に常駐して記録を続けます。終了は通知領域のメニューか、設定画面の「Worklogを終了」から行います。
- 記録間隔、離席の判定時間、除外アプリ、ログイン時の自動起動は設定画面で変えられます。
- 画面の時間は、記録間隔ごとの観測から出した推定です。正確な実働時間ではありません。
- 記録するのはアプリ名・ウィンドウタイトル・離席かどうかだけです。URL、本文、キー入力、スクリーンショットは記録しません。
- 1か月より前の記録は自動で削除します。

対応アプリ、ターミナルでプロジェクト名を記録する設定、プロジェクトの分類などは[使い方の詳細](docs/usage.md)にあります。

## 開発

Rustが必要です。加えて、macOSではXcode Command Line Tools、WindowsではMicrosoft C++ Build ToolsとWebView2を入れてください。

```sh
cargo run -p worklog      # アプリを起動
cargo test --locked       # テスト
./scripts/build-app.sh    # インストーラーを target/release/bundle/ に作成
```

検証コマンドの一覧、UIテスト、配布署名は[開発ガイド](docs/development.md)、コードの構成は[ARCHITECTURE.md](ARCHITECTURE.md)にあります。
