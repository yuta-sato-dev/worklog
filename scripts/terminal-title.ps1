# Add: . "C:\path\to\worklog\scripts\terminal-title.ps1"
# Sets a stable Worklog title in Windows Terminal, PowerShell, and compatible hosts.
# The existing prompt function is wrapped and still controls the displayed prompt.

if (-not $global:WorklogPreviousPrompt) {
    if (Test-Path Function:\prompt) {
        $global:WorklogPreviousPrompt = (Get-Command prompt).ScriptBlock
    } else {
        $global:WorklogPreviousPrompt = { "PS $($executionContext.SessionState.Path.CurrentLocation)> " }
    }
}

function global:prompt {
    $worklogRoot = $null
    try {
        $worklogRoot = (& git -C (Get-Location).Path rev-parse --show-toplevel 2>$null)
    } catch {
        $worklogRoot = $null
    }

    if (-not $worklogRoot) {
        $worklogRoot = (Get-Location).Path
    }

    $escape = [string][char]27
    $bell = [string][char]7
    $safeTitle = $worklogRoot.Replace($escape, "").Replace($bell, "").Replace("`n", "").Replace("`r", "")
    $host.UI.RawUI.WindowTitle = "worklog:$safeTitle"

    & $global:WorklogPreviousPrompt
}
