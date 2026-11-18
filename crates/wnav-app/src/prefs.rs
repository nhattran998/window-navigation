//! Preferences window — Apple-minimal redesign (HIG settings.md).
//!
//! Structure: fixed-size settings window, grouped rows, small-caps group
//! headers, switches for booleans, per-row shortcut capture fields with an
//! obvious way out (Esc). One accent, generous whitespace, zero decoration.
#![cfg(target_os = "macos")]

use gpui_kit::component::switch::Switch;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::gpui::{
    div, px, size, App, AppContext as _, Context, InteractiveElement as _, IntoElement,
    ParentElement, Render, StatefulInteractiveElement as _, Styled, TitlebarOptions, Window,
    WindowBounds, WindowKind, WindowOptions,
};
use gpui_kit::open_window;

use crate::design::{metrics, typography};
use crate::keymap::{self, Capture};
use wnav_core::{Config, PresetId};

pub(crate) fn open(cx: &mut App) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(
            size(px(metrics::WINDOW_WIDTH), px(metrics::WINDOW_HEIGHT)),
            cx,
        )),
        titlebar: Some(TitlebarOptions {
            title: Some("Wnav Preferences".into()),
            ..Default::default()
        }),
        // HIG settings.md: settings windows are not resizable mini-windows.
        kind: WindowKind::PopUp,
        is_resizable: false,
        ..Default::default()
    };
    match open_window(options, cx, |_, cx| cx.new(|_| PreferencesWindow::new())) {
        Ok((handle, _)) => {
            // Window may already be closing; nothing to do on failure.
            let _ = handle.update(cx, |_, window, _| window.activate_window());
            cx.activate(true);
        }
        Err(e) => eprintln!("wnav: prefs window failed: {e}"),
    }
}

/// One capture session's state, per row.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CaptureState {
    Idle,
    Listening { preset: PresetId },
}

struct PreferencesWindow {
    cfg: Config,
    capture: CaptureState,
    /// Row of the last failure, for an inline error line (no alerts; HIG
    /// feedback.md: feedback lives in the interface).
    error: Option<String>,
}

impl PreferencesWindow {
    fn new() -> Self {
        let cfg = Config::load(&wnav_core::config::default_config_path()).unwrap_or_default();
        Self {
            cfg,
            capture: CaptureState::Idle,
            error: None,
        }
    }

    fn toggle_general(&mut self, launch_at_login: bool, cx: &mut Context<Self>) {
        self.cfg.general.launch_at_login = launch_at_login;
        self.persist_general(cx);
    }

    fn persist_general(&mut self, _cx: &mut Context<Self>) {
        let path = wnav_core::config::default_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let mut doc = raw
            .parse::<toml_edit::DocumentMut>()
            .expect("existing config parses");
        let general = doc
            .entry("general")
            .or_insert(toml_edit::table())
            .as_table_mut()
            .expect("general is a table");
        general.insert(
            "launch-at-login",
            toml_edit::value(self.cfg.general.launch_at_login),
        );
        general.insert("edge-hints", toml_edit::value(self.cfg.general.edge_hints));
        if let Err(e) = std::fs::write(&path, doc.to_string()) {
            self.error = Some(e.to_string());
        }
    }

    fn commit_capture(
        &mut self,
        preset: PresetId,
        sc: wnav_core::Shortcut,
        cx: &mut Context<Self>,
    ) {
        match keymap::save_shortcut(preset, sc) {
            Ok(cfg) => {
                self.cfg = cfg;
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        self.capture = CaptureState::Idle;
        cx.notify();
    }
}

impl Render for PreferencesWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let t = &theme;

        // Group: Shortcuts (all 17 presets, effective shortcut shown).
        let shortcuts = self.cfg.effective_shortcuts().unwrap_or_default();
        let rows: Vec<AnyElement> = shortcuts
            .iter()
            .map(|(id, sc)| {
                let listening = self.capture == CaptureState::Listening { preset: *id };
                self.shortcut_row(*id, sc.glyphs(), listening, cx)
                    .into_any_element()
            })
            .collect();

        // Group: General.
        let general = vec![
            setting_row(
                "Launch Wnav at login",
                Some("Start automatically when you sign in"),
                Switch::new("launch-login")
                    .checked(self.cfg.general.launch_at_login)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.toggle_general(*checked, cx);
                    })),
            ),
            setting_row(
                "Show hints on screen edges while dragging",
                None,
                Switch::new("edge-hints")
                    .checked(self.cfg.general.edge_hints)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.cfg.general.edge_hints = *checked;
                        this.persist_general(cx);
                    })),
            ),
        ];

        // Inline error, only when present (HIG: no alerts for recoverable state).
        let error_line = self.error.as_ref().map(|e| {
            div()
                .text_size(px(typography::CAPTION))
                .text_color(t.danger)
                .child(e.clone())
        });

        div()
            .size_full()
            .bg(t.background)
            .text_color(t.foreground)
            .on_key_down(
                cx.listener(|this, ev: &gpui_kit::gpui::KeyDownEvent, _, cx| {
                    // While listening, the next key-down is the capture (HIG
                    // modality.md: the recorder has an obvious way out — Esc).
                    if let CaptureState::Listening { preset } = this.capture {
                        match keymap::capture(&ev.keystroke) {
                            Capture::Ready(sc) => this.commit_capture(preset, sc, cx),
                            Capture::Cancelled => {
                                this.capture = CaptureState::Idle;
                                cx.notify();
                            }
                            Capture::Pending => {}
                        }
                    }
                }),
            )
            .child(
                div()
                    .px(px(metrics::PAD_OUTER))
                    .pt(px(metrics::PAD_OUTER))
                    .flex()
                    .flex_col()
                    .gap(px(metrics::GROUP_GAP))
                    .child(group(t, "Shortcuts", rows))
                    .child(group(t, "General", general))
                    .children(error_line),
            )
    }
}

impl PreferencesWindow {
    fn shortcut_row(
        &self,
        id: PresetId,
        glyphs: String,
        listening: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme().clone();
        let t = &theme;
        let is_default = !self
            .cfg
            .presets
            .0
            .contains_key(wnav_core::preset::serde_kebab(id));

        let capture_field = div()
            .id(wnav_core::preset::serde_kebab(id))
            .h(px(metrics::CAPTURE_HEIGHT))
            .min_w(px(120.0))
            .px_2()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .border_1()
            .border_color(if listening { t.primary } else { t.border })
            .bg(t.secondary)
            .cursor_pointer()
            .text_size(px(typography::CAPTION))
            .text_color(if glyphs.is_empty() {
                t.muted
            } else {
                t.foreground
            })
            .on_click(
                cx.listener(move |this, _: &gpui_kit::gpui::ClickEvent, _, cx| {
                    this.capture = CaptureState::Listening { preset: id };
                    this.error = None;
                    cx.notify();
                }),
            )
            .child(if listening {
                "Press keys… Esc to cancel".to_string()
            } else {
                glyphs
            });

        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(metrics::ROW_GAP))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(typography::BODY))
                            .child(wnav_core::preset(id).label.to_string()),
                    )
                    .children((!is_default).then(|| {
                        // Custom overrides get a quiet "custom" marker; defaults
                        // stay unlabeled (HIG: progressive disclosure).
                        div()
                            .text_size(px(typography::CAPTION))
                            .text_color(t.muted)
                            .child("custom")
                    })),
            )
            .child(capture_field)
            .into_any_element()
    }
}

// ---- Layout helpers (kept local; they encode this window's rhythm) ----------

use gpui_kit::gpui::AnyElement;

fn setting_row(title: &str, hint: Option<&str>, control: impl IntoElement) -> AnyElement {
    let hint_text = hint.map(str::to_string);
    div()
        .flex()
        .items_center()
        .justify_between()
        .py(px(metrics::ROW_GAP))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(typography::BODY))
                        .child(title.to_string()),
                )
                .children(hint_text.map(|h| {
                    div()
                        .text_size(px(typography::CAPTION))
                        .opacity(0.6)
                        .child(h)
                })),
        )
        .child(control)
        .into_any_element()
}

fn group(t: &gpui_kit::component::Theme, title: &str, rows: Vec<AnyElement>) -> AnyElement {
    gpui_kit::component::v_flex()
        .gap(px(metrics::ROW_GAP))
        .child(
            div()
                .text_size(px(typography::GROUP_HEADER))
                .text_color(t.muted)
                .child(title.to_string()),
        )
        .child(
            gpui_kit::component::v_flex()
                .rounded(px(10.0))
                .border_1()
                .border_color(t.border)
                .bg(t.secondary)
                .px(px(metrics::PAD_ROW))
                .py(px(metrics::PAD_ROW))
                .children(rows),
        )
        .into_any_element()
}
