# leptos-resize

Reusable drag-resize handle component for Leptos UIs.

Shared primitive used by `pulse-leptos-ui::ResizeHandle` and
`mullion::SplitHandle` so both ship the same drag tracking, hover
behavior, and "stays highlighted during drag" state.

## Usage

```rust
use leptos::prelude::*;
use leptos_resize::{Direction, ResizeHandle, DEFAULT_CSS};

#[component]
fn App() -> impl IntoView {
    let width = RwSignal::new(240.0);
    let start = RwSignal::new(0.0);
    view! {
        <style>{DEFAULT_CSS}</style>
        <div style:width=move || format!("{}px", width.get())>"sidebar"</div>
        <ResizeHandle
            direction=Direction::Horizontal
            on_start=Callback::new(move |_| start.set(width.get_untracked()))
            on_move=Callback::new(move |dx: f64| {
                width.set((start.get_untracked() + dx).max(120.0));
            })
        />
        <div>"content"</div>
    }
}
```

## Theming

CSS custom properties on the handle (or any ancestor):

| Variable             | Default     | Purpose                              |
|----------------------|-------------|--------------------------------------|
| `--lrh-thickness`    | `4px`       | Visible bar size                     |
| `--lrh-target`       | `8px`       | Invisible hover-target wrapper size  |
| `--lrh-color`        | `#404040`   | Bar color when idle                  |
| `--lrh-hover-color`  | `#6a6a6a`   | Bar color while hovered OR dragging  |
