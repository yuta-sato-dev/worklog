# AGENTS

## macOSのアクセシビリティ権限が繰り返し外れる問題

GitHub ReleasesのDMGは、Apple Developer証明書のSecretsが未設定のため
adhoc署名でビルドされる。adhoc署名はビルドごとに識別子が変わるため、
DMGをダウンロードしてインストールし直すたびにmacOSがアクセシビリティ権限を
再要求し、「許可したのに別のアプリとして扱われる」症状が起きる。

このMac（開発機）では、ログインキーチェーンに `Worklog Local Code Signing`
という固定の署名IDを作成済み。GitHub ReleaseのDMGではなく、次のコマンドで
ローカルビルドしたアプリを使えば、アップデートしても同じ署名になり、
権限が外れなくなる。

```sh
./scripts/build-app.sh
```

出力は `src-tauri/target/release/bundle/` にできる`.app`／DMG。
`/Applications/Worklog.app` を置き換えて使う。

`Worklog Local Code Signing` はこのMac専用のローカル署名で、他のMacへの
配布には使えない（配布用の本命はApple Developer ID署名＋notarization）。

### 権限がおかしくなったときの復旧手順

```sh
tccutil reset Accessibility jp.local.worklog
```

実行後、Worklogを起動し、「システム設定 → プライバシーとセキュリティ →
アクセシビリティ」で許可してから再起動する。
