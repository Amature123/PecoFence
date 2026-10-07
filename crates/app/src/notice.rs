//! A notice in the corner above the taskbar, styled like a fence (`pecofence_render::notice`).
//! Windows decides how long a tray balloon stays (5 s by default) and keeps none in the
//! notification centre, so the one-time rating offer (`app/rating.rs`) uses this instead: it
//! stays for [`SHOW_FOR`], longer while the pointer rests on it. It appears without taking
//! focus; a click on it is the user's input to PecoFence, so what the click opens (the Store's
//! rating dialog) may come to the front.

use crate::commands::{Command, CommandQueue};
use crate::fence_window::FenceContext;
use crate::shadow::ShadowWindow;
use pecofence_platform::window::{
    self, ClassOptions, StandardCursor, Window, WindowBuilder, WindowClass, style,
};
use pecofence_platform::{HWND, RECT, desktop, dwm, monitors, msg};
use pecofence_render::notice::{NoticeCard, NoticeHit, NoticeLayout, WIDTH};
use pecofence_render::{DesktopWindowTarget, Panel, Theme};
use std::cell::RefCell;
use std::rc::Rc;
use windows_core::Result;

const CLASS: &str = "PecoFence.Notice";
const TIMER_DISMISS: usize = 1;
/// How long the notice stays when the pointer leaves it alone.
const SHOW_FOR_MS: u32 = 30_000;
/// Re-check interval while the pointer rests on it.
const HOVER_RECHECK_MS: u32 = 1_500;
/// Gap to the work area's edge, in DIPs.
const MARGIN: f32 = 12.0;

/// What the user did with the notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeEvent {
    /// The card or its button was clicked.
    Action,
    /// Closed with ×, or left alone until it timed out.
    Dismissed,
}

struct View {
    panel: Panel,
    card: NoticeCard,
    layout: NoticeLayout,
    theme: Theme,
    dpi: u32,
    hover: NoticeHit,
}

impl View {
    fn redraw(&self) {
        let _ = self.panel.draw(self.dpi, |s, _, _| {
            self.card.draw(s, &self.layout, &self.theme, self.hover)
        });
    }

    fn hit(&self, lparam: isize) -> NoticeHit {
        let scale = self.dpi as f32 / 96.0;
        let x = msg::lo_i16(lparam) as f32 / scale;
        let y = msg::hi_i16(lparam) as f32 / scale;
        self.layout.hit(x, y)
    }
}

/// A visible notice; dropping it closes it.
pub struct Notice {
    window: Window,
    shadow: ShadowWindow,
    _target: DesktopWindowTarget,
    _view: Rc<RefCell<View>>,
    _class: WindowClass,
}

impl Notice {
    /// Shows `title`, `body` and an `action` button at the bottom-right of the primary
    /// monitor's work area. `locale` is the text's language (fonts for Han, kana, Hangul).
    /// Clicks and the timeout arrive as [`Command::Notice`].
    pub fn show(
        ctx: &FenceContext,
        title: &str,
        body: &str,
        action: &str,
        locale: &str,
    ) -> Result<Self> {
        let monitor = monitors::enumerate()
            .into_iter()
            .find(|m| m.primary)
            .ok_or_else(windows_core::Error::empty)?;
        let dpi = monitor.dpi.max(96);
        let scale = dpi as f32 / 96.0;
        let card = NoticeCard::new(title, body, action, locale)?;
        let layout = card.layout()?;
        let (w, h) = (
            (WIDTH * scale).round() as i32,
            (layout.height * scale).round() as i32,
        );
        let margin = (MARGIN * scale).round() as i32;
        let work = monitor.work_area;
        let rect = RECT {
            left: work.right - margin - w,
            top: work.bottom - margin - h,
            right: work.right - margin,
            bottom: work.bottom - margin,
        };

        let class = WindowClass::register(CLASS, ClassOptions::default())?;
        let view: Rc<RefCell<Option<Rc<RefCell<View>>>>> = Rc::new(RefCell::new(None));
        let handler_view = view.clone();
        let queue: CommandQueue = ctx.queue.clone();
        let window = WindowBuilder::new(&class)
            .title(title)
            .style(style::POPUP)
            .ex_style(style::EX_TOPMOST | style::EX_TOOLWINDOW | style::EX_NOREDIRECTIONBITMAP)
            .bounds(rect.left, rect.top, w, h)
            .create(Box::new(move |hwnd, m, wparam, lparam| {
                let view = handler_view.borrow().clone()?;
                handle(hwnd, m, wparam, lparam, &view, &queue)
            }))?;
        let hwnd = window.hwnd();
        let _ = dwm::set_corner_preference(hwnd, dwm::CornerPreference::Round);
        let _ = dwm::set_immersive_dark_mode(
            hwnd,
            matches!(ctx.theme.borrow().mode, pecofence_render::ThemeMode::Dark),
        );

        let target = ctx.stack.create_target(hwnd)?;
        let root = ctx.stack.compositor.create_container_visual();
        target.set_root(&root);
        let mut panel = Panel::new(&ctx.stack)?;
        panel.resize(w, h)?;
        root.children().insert_at_top(&panel.visual);
        let shared = Rc::new(RefCell::new(View {
            panel,
            card,
            layout,
            theme: *ctx.theme.borrow(),
            dpi,
            hover: NoticeHit::None,
        }));
        shared.borrow().redraw();
        *view.borrow_mut() = Some(shared.clone());

        let mut shadow = ShadowWindow::create(&ctx.shadow_class, ctx.shadow_style.get())?;
        shadow.update(rect, dpi);
        window.show_no_activate();
        desktop::show_no_activate(shadow.hwnd());
        let _ = desktop::insert_after(shadow.hwnd(), hwnd);
        window.set_timer(TIMER_DISMISS, SHOW_FOR_MS);
        Ok(Self {
            window,
            shadow,
            _target: target,
            _view: shared,
            _class: class,
        })
    }
}

impl Drop for Notice {
    fn drop(&mut self) {
        self.shadow.hide();
        self.window.hide();
    }
}

fn handle(
    hwnd: HWND,
    m: u32,
    wparam: usize,
    lparam: isize,
    view: &Rc<RefCell<View>>,
    queue: &CommandQueue,
) -> Option<isize> {
    let set_hover = |hit: NoticeHit| {
        let mut v = view.borrow_mut();
        if v.hover != hit {
            v.hover = hit;
            v.redraw();
        }
    };
    match m {
        msg::WM_MOUSEMOVE => {
            let hit = view.borrow().hit(lparam);
            set_hover(hit);
            window::track_mouse_leave(hwnd);
            Some(0)
        }
        msg::WM_MOUSELEAVE => {
            set_hover(NoticeHit::None);
            Some(0)
        }
        msg::WM_SETCURSOR => {
            window::set_standard_cursor(StandardCursor::Hand);
            Some(1)
        }
        msg::WM_LBUTTONUP => {
            let event = match view.borrow().hit(lparam) {
                NoticeHit::Close => NoticeEvent::Dismissed,
                NoticeHit::None => return Some(0),
                NoticeHit::Card | NoticeHit::Button => NoticeEvent::Action,
            };
            window::kill_timer(hwnd, TIMER_DISMISS);
            queue.push(Command::Notice(event));
            Some(0)
        }
        msg::WM_TIMER if wparam == TIMER_DISMISS => {
            if view.borrow().hover != NoticeHit::None {
                window::set_timer(hwnd, TIMER_DISMISS, HOVER_RECHECK_MS);
            } else {
                window::kill_timer(hwnd, TIMER_DISMISS);
                queue.push(Command::Notice(NoticeEvent::Dismissed));
            }
            Some(0)
        }
        _ => None,
    }
}
