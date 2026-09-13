#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if ! cargo tauri --version >/dev/null 2>&1; then
  echo 'Tauri CLIをインストールしています…'
  cargo install tauri-cli --version 2.11.4 --locked
fi

case "$(uname -s)" in
  Darwin) bundles='dmg' ;;
  MINGW*|MSYS*|CYGWIN*) bundles='nsis' ;;
  *)
    echo 'この配布スクリプトはmacOSまたはWindows（Git Bash）で実行してください。' >&2
    exit 1
    ;;
esac

cargo tauri build --bundles "$bundles" -- --locked
echo 'インストーラー: src-tauri/target/release/bundle/'
