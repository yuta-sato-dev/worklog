# Add: source /absolute/path/to/scripts/terminal-title.zsh
# OSC 2 works in Terminal, iTerm2, Ghostty and other compatible terminals.
# Use an explicit marker so generic terminal titles are never mistaken for paths.
autoload -Uz add-zsh-hook
_worklog_title() {
  local worklog_root
  worklog_root=$(git -C "$PWD" rev-parse --show-toplevel 2>/dev/null) || worklog_root="$PWD"
  local worklog_dir="${worklog_root//[$'\e\a\n\r']/}"
  printf '\033]2;worklog:%s\007' "$worklog_dir"
}
add-zsh-hook chpwd _worklog_title
add-zsh-hook precmd _worklog_title
_worklog_title
