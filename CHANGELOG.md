# Changelog

## Unreleased

- Liquid Glass reworked after kube.io's refraction model: the face of a fence stays perfectly clear and all the bending sits in the rim, where a convex squircle bezel refracts the wallpaper with Snell's law (index 1.5). The very edge mirrors the backdrop like thick glass, and the magnification eases out over 40 DIP so no inner frame shows between the rim and the clear face. A thin specular ring is filled with the refracted wallpaper's colour, strongly saturated. The old material's whole-panel warp, blur and colour fringes are gone, and Liquid Glass corners are now 16 DIP. Moving a fence draws faster (one displacement pass instead of three), and GPU memory peaks lower while arranging fences (about 160 instead of 450 MB with 7 fences on a 4K monitor).
- Fences line up when resized, whatever their icon size, spacing or view: a dragged edge, top and bottom included, snaps to other fences' edges (level, or a gap apart) and to the work area's edges, with alignment guides, also with 「调整大小时保持为整数个图标」 on. Away from them that option still sizes to whole columns and rows; at a height of no whole rows the rows share the space left over like the columns share the width, so a fence still shows whole icons only. Dragged edges, expanding a rolled fence and the automatic size changes stop exactly a gap short of the next fence instead of a whole row short. Switching the option on, a late DPI change at logon, tab changes and restoring a layout no longer re-round sizes; changing a fence's icon size, spacing, view or label lines still does (#1).
- Tab names no longer vanish on a narrow fence: squeezed tab pills give up their padding (down to 8 DIP) before a caption is shortened with an ellipsis.
- Alignment guides while dragging (「显示对齐参考线」, `snapping.guideLines`, on for new configurations): thin white lines along the edges a moved or resized fence lines up with, and along the screen's centre line when it snapped there.
- Dragged fences also snap to the screen's centre lines (centred horizontally / vertically on the work area). Holding Alt while dragging or resizing moves freely, without snapping.
- When a dragged fence's title is over another fence's title, that fence gets the white outline: releasing there adds the dragged fence to it as a tab. The outline used to disappear in that spot, which looked like a glitch.
- The drop outline no longer stays on screen after a Liquid Glass drag is cancelled with Esc or a right-click, and no longer flashes on top of the dragged fence every two seconds (the desktop-layer check took it for a foreign window).

## 0.1.1

- Switching virtual desktops no longer blanks the fences: they slide out and in with the desktop, content and glass together, instead of disappearing for the whole animation and popping back afterwards. The other desktops' wallpapers are prepared in the background, so a fence already shows the new desktop's picture as it slides in (about 33 MB more per extra 4K wallpaper). During taskbar-thumbnail Aero Peek fences now fade like other windows.
- Less memory with Liquid Glass: on a 4K monitor with 7 fences the idle private working set drops from about 97 to 40 MB and GPU memory from about 200 to 80 MB. GPU caches are released once drawing goes quiet, the full-resolution wallpaper is kept only on the GPU, and fence shadows no longer hold their bitmap after it is shown.
- Fences no longer grow into each other: expanding a rolled fence, changing the icon size, auto height and the cell snap stop a snapping gap short of the next fence (the rest of the items scroll), and a dragged edge stops at a neighbouring fence. A fence dropped onto another one moves the shortest way to a free spot (an outline shows where while you drag), and overlaps a layout already has are pulled apart at startup (larger fences stay, smaller ones move; with no free spot a fence stays put) (#2).
- Changing a fixed-height fence's icon size fits its height to the content, so shrinking the icons no longer leaves rows of empty space (#2).
- Snapping settings now match Stardock Fences: 「移动栅栏时吸附对齐」 (keep fences lined up when moving), 「栅栏间距」 (space between fences, 0–32 px, back on the settings page) and 「调整大小时保持为整数个图标」 (`snapping.sizeToCells`, whole icon columns and rows, which was stored but did nothing). With it off (the default) a fence can be any size: the icon grid spreads its columns over the width, a fence keeps its width when the icon size changes (it used to grow or shift left) and a dragged side edge snaps to neighbouring fences' edges (#1, #2).
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
