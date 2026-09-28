//! Keeping fences out of each other's way: growth limits for the automatic size changes (expand,
//! icon size, auto height, column snap) and 「排列这一列」, which stacks a column of fences.

use super::*;
use crate::layout::clearance;

/// Device-px rhythm of the fence a host window shows: icon columns (none for the List /
/// Details rows, whose width is free) and whole rows below the fixed title / header part.
#[derive(Clone, Copy, Debug)]
pub(super) struct GridSteps {
    /// Column width and total horizontal padding.
    pub cols: Option<(f32, f32)>,
    pub row: f32,
    pub fixed: f32,
}

impl GridSteps {
    /// Widest whole-column width that fits in `w`.
    pub fn floor_width(&self, w: i32) -> i32 {
        match self.cols {
            Some((cell, pad)) => {
                (pad + ((w as f32 - pad) / cell).floor().max(1.0) * cell).round() as i32
            }
            None => w,
        }
    }

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
                cols: None,
                row: rm.row_h * scale,
                fixed: title_h + (rm.header_h + rm.pad_y * 2.0) * scale + 2.0,
            },
            None => GridSteps {
                cols: Some((metrics.cell_w * scale, metrics.pad_x * 2.0 * scale)),
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

    /// Lowest bottom edge host `id` at `r` may grow to without running into a fence below.
    pub(super) fn growth_limit_below(&self, id: FenceId, r: &RECT) -> Option<i32> {
        let w = self.fences.get(&id)?;
        clearance::limit_below(&self.other_fence_rects(id), r, self.gap_px(w.hwnd()))
    }

    /// Rightmost right edge host `id` at `r` may grow to without running into a fence.
    pub(super) fn growth_limit_right(&self, id: FenceId, r: &RECT) -> Option<i32> {
        let w = self.fences.get(&id)?;
        clearance::limit_right(&self.other_fence_rects(id), r, self.gap_px(w.hwnd()))
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

    /// The unlocked fences stacked with `fence`: sharing some of its width, on its monitor.
    /// Also the menu's test for whether 「排列这一列」 has anything to do (two or more).
    pub(super) fn column_of(&self, fence: FenceId) -> (Vec<(FenceId, RECT)>, Option<WorkArea>) {
        let me = self.state.host_of(fence);
        let Some(anchor) = self.fences.get(&me).map(|w| w.rect()) else {
            return (Vec::new(), None);
        };
        let centre = |r: &RECT| ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
        let (ax, ay) = centre(&anchor);
        let Some(wa) = self.work_area_at(ax, ay) else {
            return (Vec::new(), None);
        };
        let same_monitor = |r: &RECT| {
            let (x, y) = centre(r);
            x >= wa.left && x < wa.right && y >= wa.top && y < wa.bottom
        };
        let column = self
            .fences
            .iter()
            .filter(|(id, w)| {
                let r = w.rect();
                (**id == me
                    || (pecofence_platform::desktop::is_visible(w.hwnd())
                        && r.left < anchor.right
                        && r.right > anchor.left
                        && same_monitor(&r)))
                    && self.state.fence(**id).is_some_and(|f| !f.locked)
            })
            .map(|(id, w)| (*id, w.rect()))
            .collect();
        (column, Some(wa))
    }

    /// 「排列这一列」: the fences of [`Self::column_of`] get one width (the widest, in whole
    /// columns) and one edge (left, or right for a column in the right half of the screen), and
    /// are stacked top-down from the highest one with the snapping gap. Empty rows go (a fence
    /// shrinks to its content but never grows), and the heights are squeezed when the stack
    /// would not fit on the screen.
    pub(super) fn arrange_column(&mut self, fence: FenceId) {
        let (mut column, Some(wa)) = self.column_of(fence) else {
            return;
        };
        if column.len() < 2 {
            return;
        }
        column.sort_by_key(|(_, r)| (r.top, r.left));
        let width = column
            .iter()
            .map(|(_, r)| r.right - r.left)
            .max()
            .unwrap_or(0);
        let left = column.iter().map(|(_, r)| r.left).min().unwrap_or(0);
        let right = column.iter().map(|(_, r)| r.right).max().unwrap_or(0);
        let align_right = (left + right) / 2 > (wa.left + wa.right) / 2;
        let top0 = column[0].1.top;
        // Pass 1: width and edge, then each fence's height: its content, but never taller than
        // it was (a full fence would otherwise take the whole column), one row at least.
        struct Slot {
            id: FenceId,
            rolled: bool,
            steps: GridSteps,
            want: i32,
            min: i32,
            gap: i32,
        }
        let mut slots = Vec::new();
        for (id, r) in column {
            let Some(w) = self.fences.get(&id) else {
                continue;
            };
            let scale = monitors::dpi_for_window(w.hwnd()).max(96) as f32 / 96.0;
            let Some(steps) = self.grid_steps(id, scale) else {
                continue;
            };
            let fw = steps.floor_width(width);
            let x = if align_right { right - fw } else { left };
            // Width first, so the fitting height below is measured at the new column count.
            w.set_bounds(RECT {
                left: x,
                top: r.top,
                right: x + fw,
                bottom: r.bottom,
            });
            let rolled = w.is_rolled();
            let h = r.bottom - r.top;
            let (want, min) = if rolled {
                (h, h)
            } else {
                let fit = w.fit_report().map_or(h, |f| f.fitting_height_px);
                let min = steps.floor_height(0);
                (fit.min(h).max(min), min)
            };
            slots.push(Slot {
                id,
                rolled,
                steps,
                want,
                min,
                gap: self.gap_px(w.hwnd()),
            });
        }
        // Pass 2: squeeze the heights when the stack would run past the work-area bottom.
        let gaps: i32 = slots.iter().skip(1).map(|s| s.gap).sum();
        let heights = clearance::share_heights(
            &slots.iter().map(|s| (s.want, s.min)).collect::<Vec<_>>(),
            wa.bottom - top0 - gaps,
        );
        // Pass 3: stack them top-down.
        let mut top = top0;
        for (slot, h) in slots.iter().zip(heights) {
            let Some(w) = self.fences.get(&slot.id) else {
                continue;
            };
            let h = if slot.rolled {
                h
            } else {
                slot.steps.floor_height(h).min(h).max(slot.min)
            };
            if top + h > wa.bottom && top != top0 {
                break; // no room left even at one row each: the rest stay where they are
            }
            let r = w.rect();
            let rect = RECT {
                left: r.left,
                top,
                right: r.right,
                bottom: top + h,
            };
            w.set_bounds(rect);
            let expanded = if slot.rolled {
                w.expanded_height_px()
            } else {
                w.apply_height(h, false);
                h
            };
            self.state
                .set_fence_bounds(slot.id, rect, slot.rolled, expanded);
            top = rect.bottom + slot.gap;
        }
        self.schedule_save();
    }
}
