//! Reusable drag-resize handle for Leptos UIs.
//!
//! Provides a single component, [`ResizeHandle`], plus a small bundled
//! CSS stylesheet ([`DEFAULT_CSS`]). Callbacks emit pixel deltas from
//! drag start so consumers can apply whatever commit logic they want
//! (writing `style.width`, updating flex-basis ratios, persisting to
//! storage, etc.).
//!
//! The handle owns its visual chrome (hover-target wrapper + visible
//! bar) and its drag-state class toggle, so behavior stays consistent
//! across consumers: hovering paints the bar in the highlight color,
//! and that highlight is held for the full duration of an active drag
//! even when the cursor leaves the handle bounds.
//!
//! # Example
//!
//! ```ignore
//! use leptos::prelude::*;
//! use leptos_resize::{Direction, ResizeHandle};
//!
//! #[component]
//! fn MyPanel() -> impl IntoView {
//!     let width = RwSignal::new(240.0);
//!     let start = RwSignal::new(0.0);
//!     view! {
//!         <div style:width=move || format!("{}px", width.get())>"sidebar"</div>
//!         <ResizeHandle
//!             direction=Direction::Horizontal
//!             on_start=Callback::new(move |_| start.set(width.get_untracked()))
//!             on_move=Callback::new(move |dx: f64| {
//!                 width.set((start.get_untracked() + dx).max(120.0));
//!             })
//!             on_end=Callback::new(|_: f64| { /* persist if needed */ })
//!         />
//!         <div>"content"</div>
//!     }
//! }
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

/// Drag axis. Horizontal = handle reads `clientX` (col-resize cursor);
/// Vertical = handle reads `clientY` (row-resize cursor).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

/// DOM id of the per-drag global cursor lock `<style>` element. Public
/// for advanced consumers that want to coordinate; not normally needed.
pub const GLOBAL_CURSOR_STYLE_ID: &str = "__leptos_resize_cursor_lock";

/// Drag-tracking hook for consumers that want to roll their own
/// visual element (positioned/styled differently than [`ResizeHandle`]
/// does) but reuse the same drag loop, dragging-signal, and global
/// cursor lock behavior.
///
/// Returns a closure that should be attached to the visual element's
/// `on:mousedown`. The supplied `dragging` signal flips `true` for the
/// duration of the drag — wire it to your element's `class:dragging`
/// attribute (or whatever your style system uses) so the bar can stay
/// highlighted through the whole drag, even when the cursor leaves
/// the handle bounds.
///
/// Callbacks are signed pixel deltas from the drag-start position, on
/// the axis named by `direction` (X for Horizontal, Y for Vertical).
pub fn use_drag(
    dragging: RwSignal<bool>,
    direction: Direction,
    on_start: Option<Callback<()>>,
    on_move: Option<Callback<f64>>,
    on_end: Option<Callback<f64>>,
) -> impl Fn(MouseEvent) + Clone + 'static {
    move |ev: MouseEvent| {
        ev.prevent_default();

        let start = match direction {
            Direction::Horizontal => ev.client_x() as f64,
            Direction::Vertical => ev.client_y() as f64,
        };
        let dir = direction;

        if let Some(cb) = on_start {
            cb.run(());
        }
        dragging.set(true);

        let document = web_sys::window()
            .expect("window")
            .document()
            .expect("document");

        let cursor = match dir {
            Direction::Horizontal => "col-resize",
            Direction::Vertical => "row-resize",
        };
        if document.get_element_by_id(GLOBAL_CURSOR_STYLE_ID).is_none() {
            if let Ok(style) = document.create_element("style") {
                let _ = style.set_attribute("id", GLOBAL_CURSOR_STYLE_ID);
                style.set_text_content(Some(&format!(
                    "*{{cursor:{}!important;user-select:none!important;}}\
                     iframe{{pointer-events:none!important;}}",
                    cursor
                )));
                if let Some(head) = document.head() {
                    let _ = head.append_child(&style);
                }
            }
        }

        let closures: Rc<
            RefCell<
                Option<(
                    Closure<dyn FnMut(MouseEvent)>,
                    Closure<dyn FnMut(MouseEvent)>,
                )>,
            >,
        > = Rc::new(RefCell::new(None));

        let closures_for_up = closures.clone();
        let doc_for_up = document.clone();

        let mousemove_cb = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            let cur = match dir {
                Direction::Horizontal => ev.client_x() as f64,
                Direction::Vertical => ev.client_y() as f64,
            };
            if let Some(cb) = on_move {
                cb.run(cur - start);
            }
        });

        let mouseup_cb = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            let cur = match dir {
                Direction::Horizontal => ev.client_x() as f64,
                Direction::Vertical => ev.client_y() as f64,
            };
            if let Some((m, u)) = closures_for_up.borrow_mut().take() {
                let _ = doc_for_up.remove_event_listener_with_callback(
                    "mousemove",
                    m.as_ref().unchecked_ref(),
                );
                let _ = doc_for_up.remove_event_listener_with_callback(
                    "mouseup",
                    u.as_ref().unchecked_ref(),
                );
            }
            if let Some(style_el) = doc_for_up.get_element_by_id(GLOBAL_CURSOR_STYLE_ID) {
                style_el.remove();
            }
            dragging.set(false);
            if let Some(cb) = on_end {
                cb.run(cur - start);
            }
        });

        document
            .add_event_listener_with_callback(
                "mousemove",
                mousemove_cb.as_ref().unchecked_ref(),
            )
            .expect("add mousemove");
        document
            .add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref())
            .expect("add mouseup");

        *closures.borrow_mut() = Some((mousemove_cb, mouseup_cb));
    }
}

/// Drag-resize handle component.
///
/// Renders a hover-target wrapper div containing a visible bar. The
/// wrapper carries `lrh lrh-{horizontal,vertical}` classes (plus
/// `lrh-dragging` while a drag is in progress) and the bar carries
/// `lrh-bar`. See [`DEFAULT_CSS`] for the bundled stylesheet, which
/// can be themed via CSS custom properties (see crate-level docs).
///
/// All three lifecycle callbacks are optional. Pixel deltas in
/// `on_move` / `on_end` are signed (negative = cursor moved up/left
/// from drag-start, positive = down/right).
#[component]
pub fn ResizeHandle(
    /// Drag axis. Static — direction is captured on each drag start.
    #[prop(into, optional)]
    direction: Direction,
    /// Fired on mousedown, just before the drag loop installs its
    /// document-level listeners. Use this to snapshot whatever state
    /// you want to add the delta to in `on_move`.
    #[prop(optional, into)]
    on_start: Option<Callback<()>>,
    /// Fired on each mousemove during drag. Argument: signed pixel
    /// delta from the drag-start position.
    #[prop(optional, into)]
    on_move: Option<Callback<f64>>,
    /// Fired once on mouseup. Argument: final signed pixel delta.
    #[prop(optional, into)]
    on_end: Option<Callback<f64>>,
) -> impl IntoView {
    let dragging = RwSignal::new(false);
    let on_mousedown = use_drag(dragging, direction, on_start, on_move, on_end);

    let class = move || {
        let axis = match direction {
            Direction::Horizontal => "lrh-horizontal",
            Direction::Vertical => "lrh-vertical",
        };
        if dragging.get() {
            format!("lrh {} lrh-dragging", axis)
        } else {
            format!("lrh {}", axis)
        }
    };

    view! {
        <div class=class on:mousedown=on_mousedown>
            <div class="lrh-bar" />
        </div>
    }
}

/// Bundled default stylesheet. Inject once via a `<style>` element at
/// the consuming app's root, or copy-paste into your existing stylesheet.
///
/// Themable via CSS custom properties on the handle (or its ancestor):
///   * `--lrh-thickness` — visible bar size (default `4px`)
///   * `--lrh-target` — hover-target wrapper size (default `8px`)
///   * `--lrh-color` — bar color when idle (default neutral grey)
///   * `--lrh-hover-color` — bar color while hovered or dragging
pub const DEFAULT_CSS: &str = r#"
.lrh {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}
.lrh-horizontal {
    cursor: col-resize;
    width: var(--lrh-target, 8px);
}
.lrh-vertical {
    cursor: row-resize;
    height: var(--lrh-target, 8px);
}
.lrh-bar {
    background: var(--lrh-color, #404040);
    pointer-events: none;
    transition: background 150ms ease;
}
.lrh-horizontal .lrh-bar {
    width: var(--lrh-thickness, 4px);
    height: 100%;
}
.lrh-vertical .lrh-bar {
    height: var(--lrh-thickness, 4px);
    width: 100%;
}
.lrh:hover .lrh-bar,
.lrh.lrh-dragging .lrh-bar {
    background: var(--lrh-hover-color, #6a6a6a);
}
"#;
