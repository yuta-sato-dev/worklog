# Worklog design

This system applies to the dashboard and settings views within the existing single-page app.

## Direction

Modern-minimal, utilitarian. Keep the existing Cobalt accent and native Japanese typography. The dashboard uses a compact app shell: navigation rail, one toolbar, an estimated time metrics strip, activity table, and project summary. The report view uses the same shell for estimated day totals and project breakdowns. Settings uses grouped form rows in the same shell. No hero, marketing copy, decorative imagery, or animated charts. Estimated time must be labelled as 推定.

## Shared system

`tokens.css` is authoritative and is copied to `ui/tokens.css` for the static Tauri bundle. Use semantic color tokens. Headings are upright, 16–24px. Body text is 13–14px; mono is reserved for times/counts. Four-point spacing scale, 6px controls, 10px panels. Dark text and muted text must retain 4.5:1 contrast on light surfaces. Use native controls and immediate visible focus outlines. Motion is off. Hover changes background only. No remote font dependencies.

All views share the compact sidebar, status, button styles, colors, type, and content width. The desktop window minimum width is 720px; in narrow desktop windows at 800px and below, navigation becomes horizontal while the activity table remains a column-aligned table. All controls have accessible names. Observations are samples; grouped timeline durations and report totals are shown only as estimated time, never as exact working time.

## Interaction

- Dashboard: date navigation, newest-first grouped timeline, search and app/project filters, record classification with selectable title segments and match counts, editable memo export.
- Report: date-linked estimated work and idle totals, project totals, and app/title breakdowns.
- Settings: explicit save for collection preferences; separate immediate login-start switch; clear inline errors. Preserve unsaved values during background refresh.
- Closing the window hides the app; the menu-bar/tray icon reopens it. Explicit quit is available in settings and the tray.

## Exports

### CSS

Import `tokens.css`. It contains all runtime tokens; dashboard.css refers to them.

### Tailwind v4

```css
@theme {
  --color-paper: oklch(98% 0.006 258);
  --color-ink: oklch(25% 0.022 258);
  --color-accent: oklch(47% 0.21 264);
  --color-muted: oklch(48% 0.025 258);
  --spacing-md: 1rem;
  --radius-control: 6px;
}
```

### DTCG

```json
{"color":{"paper":{"$type":"color","$value":"oklch(98% 0.006 258)"},"ink":{"$type":"color","$value":"oklch(25% 0.022 258)"},"accent":{"$type":"color","$value":"oklch(47% 0.21 264)"}},"space":{"md":{"$type":"dimension","$value":{"value":16,"unit":"px"}}}}
```

### shadcn/ui

```css
:root {
  --background: oklch(98% 0.006 258);
  --foreground: oklch(25% 0.022 258);
  --primary: oklch(47% 0.21 264);
  --primary-foreground: oklch(100% 0 0);
  --muted: oklch(95% 0.025 264);
  --muted-foreground: oklch(48% 0.025 258);
  --border: oklch(88% 0.012 258);
  --ring: oklch(47% 0.21 264);
  --radius: 6px;
}
```
