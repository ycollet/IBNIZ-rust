mod audio;

use std::{
    fs,
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, Key, Modifiers, TextureHandle};
use ibniz::{HEIGHT, Program, Vm, WIDTH};

use crate::Options;
use audio::Audio;

pub fn launch(options: Options, program: Program) -> eframe::Result {
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "IBNIZ Rust",
        native,
        Box::new(move |_cc| Ok(Box::new(App::new(options, program)))),
    )
}

struct App {
    source: String,
    saved_source: String,
    path: String,
    vm: Vm,
    running: bool,
    editor: bool,
    help: bool,
    audio: Option<Audio>,
    volume: f32,
    texture: Option<TextureHandle>,
    error: Option<String>,
    compile_error: Option<String>,
    notice: String,
    budget: usize,
    next_tick: Instant,
    last_instructions: usize,
    slow_video: bool,
    slow_audio: bool,
    cursor: usize,
}

impl App {
    fn new(options: Options, program: Program) -> Self {
        let (audio, error) = if options.audio {
            match Audio::new() {
                Ok(audio) => (Some(audio), None),
                Err(error) => (
                    None,
                    Some(format!(
                        "Audio unavailable: {error}. Video and editing remain available."
                    )),
                ),
            }
        } else {
            (None, None)
        };
        Self {
            saved_source: options.source.clone(),
            source: options.source,
            path: options
                .path
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "untitled.ib".into()),
            vm: Vm::new(program),
            running: options.autorun,
            editor: true,
            help: false,
            audio,
            volume: 0.2,
            texture: None,
            error,
            compile_error: None,
            notice: String::new(),
            budget: options.budget,
            next_tick: Instant::now(),
            last_instructions: 0,
            slow_video: false,
            slow_audio: false,
            cursor: 0,
        }
    }

    fn toggle_running(&mut self) {
        self.running = !self.running;
        self.next_tick = Instant::now();
        if let Some(audio) = &mut self.audio {
            audio.clear();
        }
    }

    fn reset(&mut self) {
        self.vm.reset();
        self.next_tick = Instant::now();
        self.texture = None;
        if let Some(audio) = &mut self.audio {
            audio.clear();
        }
    }

    fn recompile(&mut self) {
        match Program::compile(&self.source) {
            Ok(program) => {
                self.vm.replace_program(program);
                self.compile_error = None;
            }
            Err(error) => self.compile_error = Some(error.to_string()),
        }
    }

    fn save(&mut self) {
        match fs::write(&self.path, &self.source) {
            Ok(()) => {
                self.saved_source = self.source.clone();
                self.notice = format!("Saved {}", self.path);
            }
            Err(error) => self.error = Some(format!("Cannot save {}: {error}", self.path)),
        }
    }

    fn load(&mut self) {
        match fs::read_to_string(&self.path) {
            Ok(source) => match Program::compile(&source) {
                Ok(program) => {
                    self.source = source;
                    self.saved_source = self.source.clone();
                    self.vm = Vm::new(program);
                    self.compile_error = None;
                    self.next_tick = Instant::now();
                    self.texture = None;
                    if let Some(audio) = &mut self.audio {
                        audio.clear();
                    }
                    self.notice = format!("Loaded {}", self.path);
                }
                Err(error) => self.error = Some(format!("Cannot load {}: {error}", self.path)),
            },
            Err(error) => self.error = Some(format!("Cannot load {}: {error}", self.path)),
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F12)) {
            self.help = !self.help;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F1)) {
            self.toggle_running();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F2)) {
            self.reset();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
            self.editor = !self.editor;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S)) {
            self.save();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.help {
                self.help = false;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        if self.running && self.compile_error.is_none() && Instant::now() >= self.next_tick {
            let frame = self.vm.render_frame(self.budget);
            self.last_instructions = frame.instructions;
            self.slow_video = !frame.video_complete && !self.vm.status().stopped;
            self.slow_audio = !frame.audio_complete && self.vm.status().audio_enabled;
            let image = egui::ColorImage::from_rgb([WIDTH, HEIGHT], &frame.rgb);
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::NEAREST);
            } else {
                self.texture =
                    Some(ctx.load_texture("ibniz-video", image, egui::TextureOptions::NEAREST));
            }
            if let Some(audio) = &mut self.audio {
                audio.enqueue(&frame.audio, self.volume);
            }
            if self.vm.status().stopped {
                self.running = false;
            }
            self.next_tick += Duration::from_secs_f64(1.0 / 60.0);
            // Don't replay an unbounded backlog after dragging or suspending the window.
            if self.next_tick < Instant::now() {
                self.next_tick = Instant::now();
            }
        }
        if self.running {
            ctx.request_repaint_after(self.next_tick.saturating_duration_since(Instant::now()));
        }
    }

    fn input(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let mut word = self.vm.input() & 0x00ff_ffff;
        let clicked = response.is_pointer_button_down_on();
        let pointer = response.hover_pos();
        ui.input(|i| {
            let mut flags = 0;
            if clicked {
                flags |= 0x80;
            }
            if i.modifiers.ctrl {
                flags |= 0x40;
            }
            if i.modifiers.alt || i.modifiers.mac_cmd {
                flags |= 0x20;
            }
            if i.modifiers.shift {
                flags |= 0x10;
            }
            for (key, bit) in [
                (Key::ArrowUp, 8),
                (Key::ArrowDown, 4),
                (Key::ArrowLeft, 2),
                (Key::ArrowRight, 1),
            ] {
                if i.key_down(key) {
                    flags |= bit;
                }
            }
            word |= flags << 24;
            if let Some(position) = pointer {
                let x = ((position.x - response.rect.left()) / response.rect.width() * 256.0)
                    .clamp(0.0, 255.0) as u32;
                let y = ((position.y - response.rect.top()) / response.rect.height() * 256.0)
                    .clamp(0.0, 255.0) as u32;
                word = (word & 0xffff0000) | (y << 8) | x;
            }
            if !self.editor {
                for event in &i.events {
                    if let egui::Event::Text(text) = event {
                        for ch in text.chars() {
                            word = (word & 0xff00ffff) | ((ch as u32 & 255) << 16);
                        }
                    }
                }
            }
        });
        self.vm.set_input(word);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl App {
    fn ui(&mut self, ctx: &egui::Context) {
        self.shortcuts(ctx);
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(if self.running {
                        "Pause (F1)"
                    } else {
                        "Run (F1)"
                    })
                    .clicked()
                {
                    self.toggle_running();
                }
                if ui.button("Reset (F2)").clicked() {
                    self.reset();
                }
                ui.checkbox(&mut self.editor, "Editor (Tab)");
                if ui.button("Help (F12)").clicked() {
                    self.help = !self.help;
                }
                ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume"));
            });
            if self.editor {
                ui.horizontal(|ui| {
                    ui.label("File");
                    ui.text_edit_singleline(&mut self.path);
                    if ui.button("Load").clicked() {
                        self.load();
                    }
                    if ui.button("Save").clicked() {
                        self.save();
                    }
                    if self.source != self.saved_source {
                        ui.label("Modified");
                    }
                });
            }
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let status = self.vm.status();
            ui.label(format!(
                "T={:04X}  Video {:?} SP={:05X}  Audio {} SP={:04X}  IP={}  {} instructions/tick{}",
                status.frame & 65535, status.video_mode, status.video_stack_pointer,
                if status.audio_enabled { "mono" } else { "off (non-producing stack)" },
                status.audio_stack_pointer, status.instruction_pointer, self.last_instructions,
                if status.stopped { "  TERMINATED" } else { "" },
            ));
            if self.slow_video || self.slow_audio {
                ui.colored_label(Color32::YELLOW, format!(
                    "Instruction budget reached: video={}, audio={}. Raise --max-instructions for complex programs.",
                    if self.slow_video { "behind" } else { "ready" },
                    if self.slow_audio { "behind" } else { "ready" },
                ));
            }
            if let Some(error) = &self.compile_error {
                ui.colored_label(Color32::RED, format!("Compile error: {error}. Playback suspended until corrected."));
            }
            if let Some(audio) = &self.audio {
                let (error, underruns, dropped) = audio.status();
                if let Some(error) = error { ui.colored_label(Color32::RED, format!("Audio error: {error}")); }
                ui.small(format!("Audio callback underruns: {underruns}; discarded queued samples: {dropped}"));
            }
            if let Some(error) = &self.error {
                ui.colored_label(Color32::RED, error);
                if ui.small_button("Dismiss").clicked() { self.error = None; }
            }
            if !self.notice.is_empty() { ui.small(&self.notice); }
        });
        if self.editor {
            egui::SidePanel::left("editor").default_width(450.0).resizable(true).show(ctx, |ui| {
                ui.heading("Live IBNIZ source");
                ui.small("Edits preserve VM state. F2 resets memory and time.");
                let source_id = egui::Id::new("source");
                if ui.memory(|m| m.has_focus(source_id)) {
                    let increment = ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::ArrowUp));
                    let decrement = ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::ArrowDown));
                    if (increment || decrement) && adjust_digit(&mut self.source, self.cursor, increment) {
                        self.recompile();
                    }
                }
                egui::ScrollArea::both().show(ui, |ui| {
                    let output = egui::TextEdit::multiline(&mut self.source).code_editor()
                            .desired_width(f32::INFINITY).desired_rows(28)
                            .id(source_id).show(ui);
                    if let Some(cursor) = output.cursor_range { self.cursor = cursor.primary.index; }
                    if output.response.changed() { self.recompile(); }
                });
                ui.separator();
                ui.small("Clipboard, selection, undo/redo and word navigation use platform editor shortcuts.");
                ui.small("Hide the editor to send typed characters to the VM's U instruction.");
                ui.small("Ctrl+Up/Down adjusts the hexadecimal digit under the cursor, with carry.");
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let size = ui.available_size().min_elem().max(1.0);
            if let Some(texture) = &self.texture {
                let response = ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(egui::vec2(size, size))
                        .sense(egui::Sense::click_and_drag()),
                );
                self.input(ui, &response);
            } else {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click_and_drag());
                ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
                self.input(ui, &response);
            }
        });
        egui::Window::new("IBNIZ Help (F12)")
            .open(&mut self.help)
            .default_width(520.0)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.heading("Keyboard shortcuts");
                ui.label("F12: show / hide help\nEsc: close help, or exit when help is hidden\nF1: run / pause\nF2: reset memory, stacks and time\nTab: show / hide editor\nCtrl/Cmd+S: save\nCtrl+Up / Down: adjust the hex digit under the cursor");
                ui.label("Use platform-standard shortcuts for clipboard, selection, undo/redo and word navigation.");
                ui.separator();
                ui.heading("IBNIZ instruction reference");
                ui.label("Numbers are uppercase hexadecimal, using 16.16 fixed point. Separate numbers with spaces or commas; use a dot for fractions.");
                ui.monospace(
                    "0-F.  number           \\     comment to end of line\n\
                     + - * / %              arithmetic\n\
                     q s a                  sqrt, sin, atan2\n\
                     & | ^ ~                AND, OR, XOR, NOT\n\
                     r l                    rotate right, shift left\n\
                     < > =                  negative, positive, zero tests\n\
                     d p x v                duplicate, drop, swap, rotate three\n\
                     ) ( z                  pick, bury, stack address\n\
                     M w T                  media switch, coordinates/time, stop\n\
                     @ !                    memory load, store\n\
                     ? : ;                  if, else, endif\n\
                     X L i j                counted loop, end, loop indices\n\
                     [ ] J                  do, while, jump\n\
                     { } V                  define subroutine, return, visit\n\
                     R P                    from / to return stack\n\
                     U                      user input (MMKK.YYXX)\n\
                     $ G                    start data segment, read data bits",
                );
                ui.separator();
                ui.label("Video starts in TYX mode; T mode is detected automatically. With M, put video code first and audio code second.");
                ui.monospace("^xp M d3r15&*");
                ui.label("Edits recompile without resetting VM state. Hide the editor to send typed characters to U. Mouse position and buttons are taken from the video display.");
                ui.hyperlink_to("Full upstream language manual", "https://github.com/viznut/IBNIZ/blob/master/src/ibniz.txt");
            });
        self.tick(ctx);
    }
}

fn adjust_digit(source: &mut String, cursor: usize, increment: bool) -> bool {
    let mut chars: Vec<_> = source.chars().collect();
    if cursor >= chars.len() {
        return false;
    }
    let mut position = cursor;
    let mut changed = false;
    loop {
        let ch = chars[position];
        if ch == '.' && chars.get(position + 1) != Some(&'.') {
            if position == 0 {
                break;
            }
            position -= 1;
            continue;
        }
        if !matches!(ch, '0'..='9' | 'A'..='F') {
            break;
        }
        let digit = ch.to_digit(16).expect("hexadecimal digit");
        let next = if increment {
            (digit + 1) & 15
        } else {
            digit.wrapping_sub(1) & 15
        };
        chars[position] = char::from_digit(next, 16)
            .expect("hexadecimal digit")
            .to_ascii_uppercase();
        changed = true;
        let carry = if increment { digit == 15 } else { digit == 0 };
        if !carry || position == 0 {
            break;
        }
        position -= 1;
    }
    if changed {
        *source = chars.into_iter().collect();
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(
            Options {
                source: "^xp M d3r15&*".into(),
                path: None,
                autorun: false,
                audio: false,
                headless: false,
                frames: 1,
                output: "output".into(),
                wav: None,
                budget: 1_000_000,
            },
            Program::compile("^xp M d3r15&*").unwrap(),
        )
    }

    #[test]
    fn hexadecimal_tweaks_carry_and_preserve_unicode() {
        let mut text = "FF.FF".to_owned();
        assert!(adjust_digit(&mut text, 4, true));
        assert_eq!(text, "00.00");
        assert!(adjust_digit(&mut text, 4, false));
        assert_eq!(text, "FF.FF");
        let mut text = "\\ lambda \u{03bb}\n9.0".to_owned();
        assert!(adjust_digit(&mut text, 13, true));
        assert!(text.ends_with("9.1"));
        assert!(!adjust_digit(&mut text, 0, true));
    }

    #[test]
    fn editor_and_video_ui_render_without_a_native_window() {
        let ctx = egui::Context::default();
        let mut app = app();
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
        app.toggle_running();
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
        assert!(app.texture.is_some());
        assert_eq!(app.vm.status().frame, 1);
        app.toggle_running();
        app.source = "invalid".into();
        app.recompile();
        assert!(app.compile_error.is_some());
        app.source = "^xp".into();
        app.recompile();
        assert!(app.compile_error.is_none());
        app.reset();
        assert_eq!(app.vm.status().frame, 0);
    }

    #[test]
    fn mouse_hover_and_typing_reach_vm_without_nested_context_locks() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.editor = false;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1080.0, 700.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(egui::pos2(200.0, 250.0)),
                    egui::Event::Text("A".into()),
                ],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| app.ui(ctx));
        }
        assert_eq!((app.vm.input() >> 16) & 255, b'A' as u32);
        assert_ne!(app.vm.input() & 0xffff, 0);
    }

    #[test]
    fn help_shortcut_toggles_without_changing_editor_or_vm_and_escape_dismisses_first() {
        for editor in [true, false] {
            let ctx = egui::Context::default();
            let mut app = app();
            app.editor = editor;
            app.vm.memory_mut()[0x123] = 42;
            let source = app.source.clone();
            for (key, visible, closing) in [
                (Key::F12, true, false),
                (Key::F12, false, false),
                (Key::F12, true, false),
                (Key::Escape, false, false),
                (Key::Escape, false, true),
            ] {
                let input = egui::RawInput {
                    events: [true, false]
                        .into_iter()
                        .map(|pressed| egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        })
                        .collect(),
                    ..Default::default()
                };
                let output = ctx.run(input, |ctx| app.ui(ctx));
                assert_eq!(app.help, visible, "{key:?}");
                let close = output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .iter()
                    .any(|command| matches!(command, egui::ViewportCommand::Close));
                assert_eq!(close, closing, "{key:?}");
                assert_eq!(app.editor, editor);
                assert_eq!(app.source, source);
                assert!(!app.running);
                assert_eq!(app.vm.status().frame, 0);
                assert_eq!(app.vm.memory()[0x123], 42);
            }
        }
    }
}
