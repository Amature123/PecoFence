# Changelog

## Unreleased

- Fences no longer grow into each other: expanding a rolled fence, changing the icon size, auto height and the whole-row snap stop a snapping gap short of the next fence (the rest of the items scroll), and a dragged edge stops at a neighbouring fence. A fence dropped onto another one moves the shortest way to a free spot (an outline shows where while you drag), and overlaps a layout already has are pulled apart at startup (larger fences stay, smaller ones move; with no free spot a fence stays put) (#2).
- Changing a fixed-height fence's icon size fits its height to the content, so shrinking the icons no longer leaves rows of empty space (#2).
- New 「整理对齐」 (Align fences) in the tray and fence menus: on that monitor, fence edges that nearly line up (within 96 DIP) snap onto one line (fences line up with each other, not with the screen edges), stacked or side-by-side neighbours that are nearly touching end up exactly the snapping gap apart, and nearby right edges get one width. Heights never change, locked fences stay put, nothing is rearranged or made to overlap, and a notification says so when everything is already tidy (#1).
- Fences can be any width: the icon grid spreads its columns over the width instead of the width snapping to whole columns. Edges line up exactly, a fence keeps its width when the icon size changes (it used to grow or shift left), and a dragged side edge snaps to neighbouring fences' edges (#1, #2).
- Smoother icon edges: icons from the shell came with straight alpha but were drawn as premultiplied, which made anti-aliased outlines look jagged, most visibly at large sizes on dark wallpapers (#2).
- 「在桌面显示文件夹」 (Show a folder on the desktop) in the tray and fence menus shows Documents, Downloads, Pictures or any chosen folder as a live fence; the UI no longer says "portal".

Found by letting an agent organise a real desktop with the CLI:

- pecofence-cli: `item move --dry-run` (method `items.planMove`) lists per item whether only the icon changes fence (`membership`), a real file moves (`fileMove`, with `destination`), nothing happens, or it is skipped (`exists`, `notAFile`). The real move now skips files whose name already exists at the destination (`skipped` + warning) instead of opening Explorer's replace dialog on the user's screen, and says `fileMove: true` when files are still on their way.
- pecofence-cli: `fence get` / `fence list` report `fit` (columns, rows, the height that shows every item, `overflow`) for the fence each window shows; `fence fit <FENCE>` applies that height.
- pecofence-cli: when the app changes a rect on its own (auto height after `fence move/resize/fit` or `fence create --rect`, whole columns and rows after `fence set iconSize/spacing/layout/labelLines`), the reply carries `adjusted {requested, applied, reason}` instead of a silently different size.
- pecofence-cli: `fence create --below/--above/--right-of/--left-of <FENCE> [--size w,h]` places a fence next to another one with aligned edges and the snapping gap, refuses rects that leave the work area and warns about overlaps.
- pecofence-cli: global `--fields id,title,rect.w` keeps only those keys (no jq needed) and `--ascii` writes non-ASCII as `\uXXXX`, for readers that decode stdout with the ANSI code page (Python on a Chinese or Japanese system, Windows PowerShell 5).
- pecofence-cli: `item list` reports `shortcutArguments`, so the same program started with other arguments no longer looks like a duplicate.
- pecofence-cli: a folder portal that has been navigated into a subfolder reports it as `portal.current`; `item list` shows that folder and `item move` to the portal puts files there.
- pecofence-cli: a hosted tab's `rect` is now its host window's (it was the tab's stale own geometry, which made fences look overlapped); snapshots report `fenceCount` for the current monitors (it used to sum every monitor layout, e.g. 21 for 7 fences).
- pecofence-cli: the hint for a rule without conditions names `describe --schema Cond` (it named a command that does not exist).
- `snapping.gapPx` now sets the gap fences keep while snapping (it was stored but ignored; the gap was always 8 DIP). `snapping.sizeToCells` and `snapping.guideLines` are still unused; setting them over the CLI returns a warning.

## 0.1.0

- New `pecofence-cli` (on PATH in the Store package as `pecofence-cli.exe`): terminals, scripts and coding agents control fences, items, rules, snapshots, config export/import and backups over a local named pipe; JSON output, `describe` for the full command and schema reference.
- pecofence-cli: `item list` reports `kind`, `ext`, `size`, `modified`, `created`, `openCount`, `lastOpened`, `shortcutTarget`, `fileName` and the filing `rule` for every item (`--kind` / `--ext` filters), so an agent can sort a desktop without opening files.
- pecofence-cli: `item rename <ITEM> <NAME>` renames the file behind an icon (extension kept unless `--keep-ext false`).
- pecofence-cli: `rule apply --dry-run` lists the moves the rules would make without making them; `rule apply` now reports the same `moves` list.
- pecofence-cli: `item move` of 20 or more desktop items takes an automatic layout snapshot first (`snapshotId`), like `fence delete` and `rule apply`.
- pecofence-cli: `watch` streams desktop events (`item.added/removed/moved`, `fence.created/deleted/changed`, heartbeat) over a long-lived pipe connection; `--fence`, `--events`, `--once` for scripts that wait for one change and then run a batch.
- pecofence-cli: offline `config check [FILE]` validates and lints a config file (rule targets, portal folders, tab hosts, `$schema`), `paths` shows where config, backups, log and crash dumps live, `log [-f] [-n]` prints or follows the app log.
- `config.json` and exports start with a `$schema` field pointing at the published JSON Schema (`describe --schema Config`).
- About 430 KB smaller `pecofence.exe`: `RUST_LOG` now takes `level` and `target=level` directives (for example `pecofence=debug`) without span or field filters, so the regex engine is no longer linked in.
- Start with Windows now follows the release you launch: an entry still pointing at an older copy (for example one left in Downloads) is re-pointed to the running executable.

## 0.0.5

- Fence corners are 8 DIP in both materials (Fluent was 4, Liquid Glass 24), matching Windows 11 windows. Title-bar controls follow.
- Settings: the Liquid Glass switch is 44×24 instead of the oversized 54×30.

## 0.0.4

- Fence context menu → Sort → "Group by date": items are shown under Today / Yesterday / This week / This month / Earlier headers in the icon, list and details layouts.
- Settings → Organizing rules → Quick add: one click creates a fence and its rule for Images, Music, Videos, Archives, Installers, or "To clean up" (installers and archives unused for 30 days; gathered, never deleted).
- New rule condition "idle days": the item was neither modified nor opened from a fence for N days. Rules using it are re-run hourly.
- New file type category "Installers": .msi/.msix/.appx packages and setup/install-named .exe files.
- Consistent app icon: Store tiles, exe icon, taskbar icon, tray icon and the settings window share the white-on-indigo mark.

## 0.0.2

- While Windows desktop icons are hidden, the special desktop items the user has enabled in Windows (Recycle Bin, This PC, User's Files, Network, Control Panel) appear in the Desktop fence: double-click opens them, the shell context menu works (Empty Recycle Bin, Properties), files dropped on the Recycle Bin are recycled, and the Recycle Bin icon follows its contents (also when files are recycled from elsewhere).
- Version bump for the Microsoft Store resubmission; package identity requires a new version per upload.
- Executables link the C runtime statically; the Visual C++ Redistributable is no longer required (Store policy 10.2.4.1).
- License changed from MIT to the Apache License 2.0; a NOTICE file accompanies the LICENSE.

## 0.0.1

- Renamed the product, executables and packages to PecoFence; existing configuration
  directories, environment overrides and startup entries are handled compatibly.
- Desktop fences, folder portals, tab groups, Explorer file operations and sorting.
- Quick Hide, Peek, Fluent and Liquid Glass themes.
- Automatic organizing rules, layout snapshots, configuration import/export and backups.
- Ten offline interface languages with live switching and Windows display-language detection.
- README and hero artwork in all ten languages under `docs/readme/`.
- Static product website in `site/`, built by `scripts/build-site.py` and published with GitHub Pages.
- Shared native/settings translation catalogs and translation validation.
- Portable x64 packaging, clean source export and draft GitHub Release workflow.
- Portable startup preserves the installed copy's Windows autostart registration.
- Independent instances use separate WebView2 profiles.

This is an initial release. Full compatibility testing across older Windows 11
versions and different graphics/display configurations is still in progress.
