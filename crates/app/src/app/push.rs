//! Push neighbours: while a fence is expanded (a click, a hover peek, a drag of its edge), the
//! fences stacked below it slide down out of its way, and slide back when it rolls up.
//!
//! The windows are moved directly and nothing is stored in the configuration: the saved
//! rectangles stay the resting layout (`resting_rect` undoes the shift for the one path that
//! persists a window's rectangle while it is displaced). A fence somebody else moved — the
//! user dragging it, `fence move` — is no longer where this pass left it, so its current
//! position becomes its new resting one.

use super::*;

/// Where one displaced fence was put: the resting y it came from and the y it was given.
#[derive(Clone, Copy)]
struct Applied {
    base_y: i32,
    y: i32,
}

#[derive(Default)]
pub(super) struct PushState {
    applied: HashMap<FenceId, Applied>,
    /// Height a fence occupies in the resting layout: its title bar for a fence that is rolled
    /// up (so an expansion counts as growth), else the height it had when first seen.
    rest_h: HashMap<FenceId, i32>,
}

impl PushState {
    /// `rect` with this pass's shift taken out, if the window is still where it put it.
    pub(super) fn resting_rect(&self, fence: FenceId, mut rect: RECT) -> RECT {
        if let Some(a) = self.applied.get(&fence)
            && a.y != a.base_y
            && rect.top == a.y
        {
            let shift = a.y - a.base_y;
            rect.top -= shift;
            rect.bottom -= shift;
        }
        rect
    }
}

struct Slot {
    id: FenceId,
    hwnd: HWND,
    rect: RECT,
    base_y: i32,
    y: i32,
    rest_h: i32,
}

/// Gives every slot its y for the heights it has now: a fence is pushed down just far enough to
/// clear each fence above it that overlaps it horizontally, keeping the gap the resting layout
/// had (at most `gap`), so a layout at rest never moves. `enabled == false` rests everything.
fn stack(slots: &mut [Slot], enabled: bool, gap: i32) {
    slots.sort_by_key(|s| (s.base_y, s.rect.left, s.id));
    for j in 0..slots.len() {
        let mut y = slots[j].base_y;
        if enabled {
            for i in 0..j {
                let (a, b) = (&slots[i], &slots[j]);
                let overlap_x = a.rect.left < b.rect.right && b.rect.left < a.rect.right;
                let resting_bottom = a.base_y + a.rest_h;
                // Only a fence that sits below `a` in the resting layout is pushed by it.
                if !overlap_x || b.base_y < resting_bottom {
                    continue;
                }
                let slack = (b.base_y - resting_bottom).min(gap);
                y = y.max(a.y + (a.rect.bottom - a.rect.top) + slack);
            }
        }
        slots[j].y = y;
    }
}

impl App {
    /// Lays the fences out for the current heights. Cheap (a rectangle read per window), so it
    /// runs on every frame and after every command. Always returns false: it moves windows
    /// directly and needs no frame of its own.
    pub(super) fn reflow_pushed(&mut self) -> bool {
        let enabled = self.state.config.settings.roll_up.push_neighbors;
        if !enabled && self.pushed.applied.is_empty() {
            return false;
        }
        let gap = self.state.config.settings.snapping.gap_px.max(0);
        let push = &mut self.pushed;
        let mut slots: Vec<Slot> = Vec::with_capacity(self.fences.len());
        for (&id, w) in &self.fences {
            let hwnd = w.hwnd();
            if !desktop::is_visible(hwnd) {
                continue;
            }
            let rect = window::window_rect(hwnd);
            let h = rect.bottom - rect.top;
            if h <= 0 {
                continue;
            }
            let base_y = match push.applied.get(&id) {
                Some(a) if rect.top == a.y => a.base_y,
                _ => rect.top,
            };
            if w.is_collapsed_at_rest() {
                push.rest_h.insert(id, h);
            } else {
                push.rest_h.entry(id).or_insert_with(|| {
                    let rolled = self.state.fence(id).is_some_and(|f| f.rolled_up);
                    if rolled { w.collapsed_height_px() } else { h }
                });
            }
            slots.push(Slot {
                id,
                hwnd,
                rect,
                base_y,
                y: base_y,
                rest_h: push.rest_h[&id],
            });
        }
        stack(&mut slots, enabled, gap);
        for s in &slots {
            if s.rect.top != s.y {
                let (w, h) = (s.rect.right - s.rect.left, s.rect.bottom - s.rect.top);
                let _ = window::set_window_bounds(s.hwnd, s.rect.left, s.y, w, h);
            }
            if s.y != s.base_y {
                push.applied.insert(
                    s.id,
                    Applied {
                        base_y: s.base_y,
                        y: s.y,
                    },
                );
            } else {
                push.applied.remove(&s.id);
            }
        }
        push.applied.retain(|id, _| self.fences.contains_key(id));
        push.rest_h.retain(|id, _| self.fences.contains_key(id));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(n: u128, x: i32, y: i32, h: i32, rest_h: i32) -> Slot {
        Slot {
            id: FenceId::from_u128(n),
            hwnd: HWND(std::ptr::null_mut()),
            rect: RECT {
                left: x,
                top: y,
                right: x + 240,
                bottom: y + h,
            },
            base_y: y,
            y,
            rest_h,
        }
    }

    fn ys(slots: &[Slot], ids: &[u128]) -> Vec<i32> {
        ids.iter()
            .map(|n| {
                slots
                    .iter()
                    .find(|s| s.id == FenceId::from_u128(*n))
                    .unwrap()
                    .y
            })
            .collect()
    }

    #[test]
    fn resting_stack_does_not_move() {
        // Three rolled fences 10 px apart, then one with a 4 px gap.
        let mut s = vec![
            slot(1, 0, 100, 36, 36),
            slot(2, 0, 146, 36, 36),
            slot(3, 0, 192, 36, 36),
            slot(4, 0, 232, 36, 36),
        ];
        stack(&mut s, true, 8);
        assert_eq!(ys(&s, &[1, 2, 3, 4]), vec![100, 146, 192, 232]);
    }

    #[test]
    fn expanded_fence_pushes_the_ones_below_and_they_come_back() {
        let mut s = vec![
            slot(1, 0, 100, 150, 36),
            slot(2, 0, 146, 36, 36),
            slot(3, 0, 192, 36, 36),
        ];
        stack(&mut s, true, 8);
        // 100 + 150 + 8, then 36 + 8 further down.
        assert_eq!(ys(&s, &[1, 2, 3]), vec![100, 258, 302]);
        s[0].rect.bottom = s[0].rect.top + 36;
        stack(&mut s, true, 8);
        assert_eq!(ys(&s, &[1, 2, 3]), vec![100, 146, 192]);
    }

    #[test]
    fn only_overlapping_columns_and_fences_below_are_pushed() {
        let mut s = vec![
            slot(1, 0, 100, 300, 36),
            slot(2, 500, 146, 36, 36), // other column
            slot(3, 0, 20, 36, 36),    // above
        ];
        stack(&mut s, true, 8);
        assert_eq!(ys(&s, &[1, 2, 3]), vec![100, 146, 20]);
    }

    #[test]
    fn a_big_gap_absorbs_the_growth() {
        let mut s = vec![slot(1, 0, 100, 150, 36), slot(2, 0, 400, 36, 36)];
        stack(&mut s, true, 8);
        assert_eq!(ys(&s, &[2]), vec![400]);
    }

    #[test]
    fn disabled_rests_everything() {
        let mut s = vec![slot(1, 0, 100, 150, 36), slot(2, 0, 146, 36, 36)];
        stack(&mut s, false, 8);
        assert_eq!(ys(&s, &[2]), vec![146]);
    }
}
