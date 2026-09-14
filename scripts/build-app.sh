#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if ! cargo tauri --version >/dev/null 2>&1; then
  echo 'Tauri CLIをインストールしています…'
  cargo install tauri-cli --version 2.11.4 --locked
fi

case "$(uname -s)" in
  Darwin)
    bundles='dmg'
    if security find-identity -v -p codesigning | grep -q '"Worklog Local Code Signing"'; then
      export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:-Worklog Local Code Signing}"
      echo "macOS署名: $APPLE_SIGNING_IDENTITY"
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*) bundles='nsis' ;;
  *)
    echo 'この配布スクリプトはmacOSまたはWindows（Git Bash）で実行してください。' >&2
    exit 1
    ;;
esac

cargo tauri build --bundles "$bundles" -- --locked
echo 'インストーラー: target/release/bundle/'
