use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use eframe::egui;

use crate::{
    hotkey::{HotkeyHandle, HOTKEY_LABEL},
    input::{self, Layout},
};

const DEFAULT_DELAY_SECS: f32 = 3.0;
const DEFAULT_INTERVAL_MS: u64 = 10;
const HOTKEY_RELEASE_DELAY: Duration = Duration::from_millis(200);

enum UiMessage {
    TypingStarted,
    TypingFinished(Result<(), String>),
}

struct PendingJob {
    cancel: Arc<AtomicBool>,
    deadline: Instant,
}

pub struct JustInputApp {
    text: String,
    delay_secs: f32,
    interval_ms: u64,
    layout: Layout,
    status: String,
    pending: Option<PendingJob>,
    typing: bool,
    ui_tx: Sender<UiMessage>,
    ui_rx: Receiver<UiMessage>,
    hotkey: Option<HotkeyHandle>,
    hotkey_rx: Option<Receiver<()>>,
}

impl Default for JustInputApp {
    fn default() -> Self {
        let (ui_tx, ui_rx) = mpsc::channel();

        Self {
            text: String::new(),
            delay_secs: DEFAULT_DELAY_SECS,
            interval_ms: DEFAULT_INTERVAL_MS,
            layout: Layout::default(),
            status: "Ready".to_owned(),
            pending: None,
            typing: false,
            ui_tx,
            ui_rx,
            hotkey: None,
            hotkey_rx: None,
        }
    }
}

impl JustInputApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn schedule_timer(&mut self, ctx: &egui::Context) {
        if self.schedule_type_after(Duration::from_secs_f32(self.delay_secs.max(0.0))) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn schedule_hotkey_type(&mut self) {
        if self.schedule_type_after(HOTKEY_RELEASE_DELAY) {
            self.status = "Typing from hotkey".to_owned();
        }
    }

    fn can_start(&mut self) -> bool {
        if self.text.is_empty() {
            self.status = "Nothing to type".to_owned();
            return false;
        }

        if self.typing {
            self.status = "Typing is already running".to_owned();
            return false;
        }

        if self.pending.is_some() {
            self.status = "Typing is already scheduled".to_owned();
            return false;
        }

        true
    }

    fn schedule_type_after(&mut self, delay: Duration) -> bool {
        if !self.can_start() {
            return false;
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let thread_cancel = Arc::clone(&cancel);
        let text = self.text.clone();
        let interval = Duration::from_millis(self.interval_ms);
        let layout = self.layout;
        let deadline = Instant::now() + delay;
        let tx = self.ui_tx.clone();

        self.pending = Some(PendingJob { cancel, deadline });
        self.status = format!("Typing in {:.1}s", delay.as_secs_f32());

        thread::spawn(move || {
            while Instant::now() < deadline {
                if thread_cancel.load(Ordering::Relaxed) {
                    return;
                }

                let remaining = deadline.saturating_duration_since(Instant::now());
                thread::sleep(remaining.min(Duration::from_millis(50)));
            }

            if thread_cancel.load(Ordering::Relaxed) {
                return;
            }

            let _ = tx.send(UiMessage::TypingStarted);
            let result =
                input::type_text(&text, interval, layout).map_err(|error| error.to_string());
            let _ = tx.send(UiMessage::TypingFinished(result));
        });

        true
    }

    fn cancel_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.cancel.store(true, Ordering::Relaxed);
            self.status = "Cancelled".to_owned();
        }
    }

    fn toggle_hotkey(&mut self) {
        if self.hotkey.is_some() {
            self.hotkey = None;
            self.hotkey_rx = None;
            self.status = "Hotkey off".to_owned();
            return;
        }

        let (tx, rx) = mpsc::channel();
        match HotkeyHandle::register(tx) {
            Ok(handle) => {
                self.hotkey = Some(handle);
                self.hotkey_rx = Some(rx);
                self.status = format!("{HOTKEY_LABEL} armed");
            }
            Err(error) => {
                self.status = error.to_string();
            }
        }
    }

    fn receive_messages(&mut self) {
        while let Ok(message) = self.ui_rx.try_recv() {
            match message {
                UiMessage::TypingStarted => {
                    self.pending = None;
                    self.typing = true;
                    self.status = "Typing".to_owned();
                }
                UiMessage::TypingFinished(Ok(())) => {
                    self.pending = None;
                    self.typing = false;
                    self.status = "Done".to_owned();
                }
                UiMessage::TypingFinished(Err(error)) => {
                    self.pending = None;
                    self.typing = false;
                    self.status = error;
                }
            }
        }

        let hotkey_events = self
            .hotkey_rx
            .as_ref()
            .map(|rx| rx.try_iter().count())
            .unwrap_or(0);

        for _ in 0..hotkey_events {
            self.schedule_hotkey_type();
        }
    }

    fn update_pending_status(&mut self) {
        let Some(pending) = self.pending.as_ref() else {
            return;
        };

        let remaining = pending.deadline.saturating_duration_since(Instant::now());
        self.status = format!("Typing in {:.1}s", remaining.as_secs_f32());
    }
}

impl eframe::App for JustInputApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_messages();

        if ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Enter)) {
            self.schedule_timer(ctx);
        }

        self.update_pending_status();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Just Input");
            ui.add_space(8.0);

            ui.add_sized(
                [ui.available_width(), 230.0],
                egui::TextEdit::multiline(&mut self.text)
                    .hint_text("Text")
                    .desired_rows(10),
            );

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Delay");
                ui.add(
                    egui::DragValue::new(&mut self.delay_secs)
                        .clamp_range(0.0..=60.0)
                        .speed(0.25)
                        .suffix(" s"),
                );

                ui.separator();

                ui.label("Interval");
                ui.add(
                    egui::DragValue::new(&mut self.interval_ms)
                        .clamp_range(0..=1000)
                        .speed(1)
                        .suffix(" ms"),
                );
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Keyboard");
                ui.radio_value(&mut self.layout, Layout::EnUs, "EN");
                ui.radio_value(&mut self.layout, Layout::RuQwerty, "RU");
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let type_enabled = !self.typing && self.pending.is_none();
                if ui
                    .add_enabled(type_enabled, egui::Button::new("Type"))
                    .clicked()
                {
                    self.schedule_timer(ctx);
                }

                if ui
                    .add_enabled(self.pending.is_some(), egui::Button::new("Cancel"))
                    .clicked()
                {
                    self.cancel_pending();
                }

                let hotkey_label = if self.hotkey.is_some() {
                    format!("Disarm {HOTKEY_LABEL}")
                } else {
                    format!("Arm {HOTKEY_LABEL}")
                };

                if ui.button(hotkey_label).clicked() {
                    self.toggle_hotkey();
                }
            });

            ui.add_space(8.0);
            ui.label(&self.status);
        });

        if self.pending.is_some() || self.typing || self.hotkey.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}
