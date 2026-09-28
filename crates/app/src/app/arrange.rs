//! Keeping fences out of each other's way: growth limits for the automatic size changes (expand,
//! icon size, auto height, row snap), moving a dropped fence off another one, pulling overlaps
//! apart at startup, and 「整理对齐」.

use super::*;
use crate::layout::clearance;

/// Device-px row rhythm of the fence a host window shows: whole rows below the fixed title /
/// header part (the width is free).
#[derive(Clone, Copy, Debug)]
pub(super) struct GridSteps {
    pub row: f32,
    pub fixed: f32,
}

impl GridSteps {
    /// Tallest whole-row height that fits in `h` (one row at least).
    pub fn floor_height(&self, h: i32) -> i32 {
        (self.fixed + ((h as f32 - self.fixed) / self.row).floor().max(1.0) * self.row).round()
            as i32
    }
}

impl App {
    /// Grid rhythm of host `id`'s shown fence at `scale`.
    pub(super) fn grid_steps(&self, id: FenceId, scale: f32) -> Option<GridSteps> {
        let shown = self.state.fence(self.state.active_tab_of(id))?;
        let metrics =
            crate::layout::GridMetrics::for_icon_size(shown.view.icon_size, shown.view.label_lines)
                .with_line_h(self.ctx.chrome.label_line_h())
                .with_spacing(shown.view.spacing);
        let title_h = (self.ctx.theme.borrow().title_height * scale).round();
        let rows = match shown.view.layout {
            ViewLayout::Icons => None,
            ViewLayout::List => Some(crate::layout::RowMetrics::list()),
            ViewLayout::Details => Some(crate::layout::RowMetrics::details()),
        };
        Some(match rows {
            Some(rm) => GridSteps {
                row: rm.row_h * scale,
                fixed: title_h + (rm.header_h + rm.pad_y * 2.0) * scale + 2.0,
            },
            None => GridSteps {
                row: metrics.cell_h * scale,
                fixed: title_h + metrics.pad_y * 2.0 * scale + 2.0,
            },
        })
    }

    /// The snapping gap (`snapping.gapPx`) in device px for `hwnd`'s monitor.
    pub(super) fn gap_px(&self, hwnd: HWND) -> i32 {
        let scale = monitors::dpi_for_window(hwnd).max(96) as f32 / 96.0;
        (settings::snap_gap_dip(&self.state.config.settings.snapping) as f32 * scale).round() as i32
    }

    /// Screen rectangles of the other fence windows that are showing.
    fn other_fence_rects(&self, id: FenceId) -> Vec<RECT> {
        self.fences
            .iter()
            .filter(|(fid, w)| **fid != id && pecofence_platform::desktop::is_visible(w.hwnd()))
            .map(|(_, w)| w.rect())
            .collect()
    }

    /// Moves host `id` to `rect` and records it.
    fn relocate(&mut self, id: FenceId, rect: RECT) {
        let Some(w) = self.fences.get(&id) else {
            return;
        };
        w.set_bounds(rect);
        let rolled = w.is_rolled();
        let expanded = if rolled {
            w.expanded_height_px()
        } else {
            rect.bottom - rect.top
        };
        self.state.set_fence_bounds(id, rect, rolled, expanded);
        self.schedule_save();
    }

    /// A fence the user dropped onto another one moves the shortest way to a free spot on its
    /// monitor (the snapping gap kept); with no free spot it stays where it was dropped.
    pub(super) fn move_out_of_overlap(&mut self, fence: FenceId) {
        let id = self.state.host_of(fence);
        let Some(w) = self.fences.get(&id) else {
            return;
        };
        let r = w.rect();
        let gap = self.gap_px(w.hwnd());
        let others = self.other_fence_rects(id);
        if !others.iter().any(|o| clearance::overlaps(&r, o, gap)) {
            return;
        }
        let Some(wa) = self.work_area_at((r.left + r.right) / 2, (r.top + r.bottom) / 2) else {
            return;
        };
        let work = RECT {
            left: wa.left,
            top: wa.top,
            right: wa.right,
            bottom: wa.bottom,
        };
        if let Some(spot) = clearance::nearest_free(&r, &others, &work, gap) {
            tracing::info!(%id, from = ?(r.left, r.top), to = ?(spot.left, spot.top), "fence moved out of an overlap");
            self.relocate(id, spot);
        }
    }

    /// Startup: fences that already overlap are pulled apart once. Larger fences stay; each
    /// smaller one that covers a fence kept so far moves to the nearest spot free of every
    /// other fence (or, failing that, of the kept ones), so the fewest and smallest move.
    pub(super) fn resolve_overlaps(&mut self) {
        let mut order: Vec<(FenceId, RECT)> =
            self.fences.iter().map(|(id, w)| (*id, w.rect())).collect();
        let area = |r: &RECT| (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
        order.sort_by_key(|(_, r)| (std::cmp::Reverse(area(r)), r.top, r.left));
        let mut kept: Vec<RECT> = Vec::new();
        for i in 0..order.len() {
            let (id, r) = order[i];
            let Some(w) = self.fences.get(&id) else {
                continue;
            };
            let gap = self.gap_px(w.hwnd());
            if !kept.iter().any(|k| clearance::overlaps(&r, k, gap)) {
                kept.push(r);
                continue;
            }
            let Some(wa) = self.work_area_at((r.left + r.right) / 2, (r.top + r.bottom) / 2) else {
                kept.push(r);
                continue;
            };
            let work = RECT {
                left: wa.left,
                top: wa.top,
                right: wa.right,
                bottom: wa.bottom,
            };
            let all: Vec<RECT> = order
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, (_, o))| *o)
                .collect();
            let spot = clearance::nearest_free(&r, &all, &work, gap)
                .or_else(|| clearance::nearest_free(&r, &kept, &work, gap));
            match spot {
                Some(spot) => {
                    tracing::info!(%id, from = ?(r.left, r.top), to = ?(spot.left, spot.top), "overlapping fence moved at startup");
                    self.relocate(id, spot);
                    order[i].1 = spot;
                    kept.push(spot);
                }
                None => kept.push(r),
            }
        }
    }

    /// Lowest bottom edge host `id` at `r` may grow to without running into a fence below.
    pub(super) fn growth_limit_below(&self, id: FenceId, r: &RECT) -> Option<i32> {
        let w = self.fences.get(&id)?;
        clearance::limit_below(&self.other_fence_rects(id), r, self.gap_px(w.hwnd()))
    }

    /// Expanding a rolled fence: the expanded height stops short of a fence placed under the
    /// title row meanwhile (the rest of the items scroll).
    pub(super) fn clamp_expand_height(&mut self, id: FenceId) {
        let Some(w) = self.fences.get(&id) else {
            return;
        };
        let r = w.rect();
        let expanded = w.expanded_height_px();
        let scale = monitors::dpi_for_window(w.hwnd()).max(96) as f32 / 96.0;
        let (Some(limit), Some(steps)) =
            (self.growth_limit_below(id, &r), self.grid_steps(id, scale))
        else {
            return;
        };
        if r.top + expanded <= limit {
            return;
        }
        let h = steps.floor_height(limit - r.top);
        if h >= expanded {
            return;
        }
        w.set_expanded_height_px(h);
        self.state.set_fence_bounds(id, r, true, h);
    }

    /// Icon-size change on a fixed-height fence: the height follows the content (no band of
    /// empty rows after shrinking the icons), growing only as far as the next fence below.
    pub(super) fn fit_height_to_content(&mut self, id: FenceId) {
        let id = self.state.host_of(id);
        if self.state.fence(id).is_none_or(|f| f.view.auto_height) {
            return;
        }
        let Some(w) = self.fences.get(&id) else {
            return;
        };
        if w.is_rolled() {
            return;
        }
        let Some(fit) = w.fit_report() else {
            return;
        };
        let r = w.rect();
        let scale = monitors::dpi_for_window(w.hwnd()).max(96) as f32 / 96.0;
        let mut h = fit.fitting_height_px;
        if let Some(limit) = self.growth_limit_below(id, &r)
            && let Some(steps) = self.grid_steps(id, scale)
        {
            let limit = limit.max(r.bottom);
            if r.top + h > limit {
                h = steps.floor_height(limit - r.top);
            }
        }
        if h == r.bottom - r.top {
            return;
        }
        let rect = w.apply_height(h, false);
        self.state.set_fence_bounds(id, rect, false, h);
        self.schedule_save();
    }

    /// 「整理对齐」: [`clearance::tidy`] over the fences showing on the monitor of `fence` (the
    /// monitor under the cursor from the tray): near-miss edges line up and small gaps even out
    /// to the snapping gap; locked fences stay and act as anchors.
    pub(super) fn tidy_align(&mut self, fence: Option<FenceId>, x: i32, y: i32) {
        let (cx, cy) = fence
            .map(|f| self.state.host_of(f))
            .and_then(|h| self.fences.get(&h))
            .map(|w| {
                let r = w.rect();
                ((r.left + r.right) / 2, (r.top + r.bottom) / 2)
            })
            .unwrap_or((x, y));
        let Some(wa) = self.work_area_at(cx, cy) else {
            return;
        };
        let on_monitor = |r: &RECT| {
            let (x, y) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
            x >= wa.left && x < wa.right && y >= wa.top && y < wa.bottom
        };
        let mut ids = Vec::new();
        let mut input = Vec::new();
        let mut hwnd = None;
        for (id, w) in &self.fences {
            let r = w.rect();
            if !pecofence_platform::desktop::is_visible(w.hwnd()) || !on_monitor(&r) {
                continue;
            }
            let Some(f) = self.state.fence(*id) else {
                continue;
            };
            let scale = monitors::dpi_for_window(w.hwnd()).max(96) as f32 / 96.0;
            let shown = self.state.fence(self.state.active_tab_of(*id)).unwrap_or(f);
            let metrics = crate::layout::GridMetrics::for_icon_size(
                shown.view.icon_size,
                shown.view.label_lines,
            )
            .with_spacing(shown.view.spacing);
            ids.push(*id);
            input.push(clearance::TidyFence {
                rect: r,
                fixed: f.locked,
                min_w: ((metrics.cell_w + metrics.pad_x * 2.0) * scale).round() as i32,
            });
            hwnd.get_or_insert(w.hwnd());
        }
        let Some(hwnd) = hwnd else {
            return;
        };
        let scale = monitors::dpi_for_window(hwnd).max(96) as f32 / 96.0;
        let work = RECT {
            left: wa.left,
            top: wa.top,
            right: wa.right,
            bottom: wa.bottom,
        };
        let reach = (TIDY_REACH_DIP * scale).round() as i32;
        let out = clearance::tidy(&input, &work, self.gap_px(hwnd), reach);
        let mut moved = 0;
        for ((id, before), after) in ids.into_iter().zip(input).zip(out) {
            if after != before.rect {
                self.relocate(id, after);
                moved += 1;
            }
        }
        tracing::info!(moved, "tidy align");
        if moved == 0
            && let Some(t) = &self.tray
        {
            t.show_info(
                "PecoFence",
                pecofence_core::i18n::text("已经很整齐了，没有需要对齐的栅栏。"),
                false,
            );
        }
    }
}

/// Edges closer than this line up in 「整理对齐」 (DIPs); farther ones are left alone.
const TIDY_REACH_DIP: f32 = 96.0;
