# Worklog design

This system applies to the dashboard and settings views within the existing single-page app.

## Direction

Modern-minimal, utilitarian. Keep the existing Cobalt accent and native Japanese typography. The dashboard uses a compact app shell: navigation rail, one toolbar, an estimated time metrics strip, activity table, and project summary. The report view uses the same shell for estimated day totals and project breakdowns. Settings uses grouped form rows in the same shell. No hero, marketing copy, decorative imagery, or animated charts. Estimated time must be labelled as 推定.

## Shared system

`ui/tokens.css` is authoritative. Use semantic color tokens. Headings are upright, 16–24px. Body text is 13–14px; mono is reserved for times/counts. Four-point spacing scale, 6px controls, 10px panels. Dark text and muted text must retain 4.5:1 contrast on light surfaces. Use native controls and immediate visible focus outlines. Motion is off. Hover changes background only. No remote font dependencies.

All views share the compact sidebar, status, button styles, colors, type, and content width. The desktop window minimum width is 720px; in narrow desktop windows at 800px and below, navigation becomes horizontal while the activity table remains a column-aligned table. All controls have accessible names. Observations are samples; grouped timeline durations and report totals are shown only as estimated time, never as exact working time.

## Interaction

- Dashboard: date navigation, newest-first grouped timeline, search and app/project filters, record classification with selectable title segments and match counts.
- Report: date-linked estimated work and idle totals, project totals, and app/title breakdowns.
- Settings: explicit save for collection preferences; separate immediate login-start switch; clear inline errors. Preserve unsaved values during background refresh.
- Closing the window hides the app; the menu-bar/tray icon reopens it. Explicit quit is available in settings and the tray.
