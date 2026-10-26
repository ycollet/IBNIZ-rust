mod audio;
mod examples;
mod nsm;
mod recording;

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, Key, Modifiers, TextureHandle};
use ibniz::{FrameRender, HEIGHT, Program, Vm, WIDTH};

use crate::Options;
use audio::Audio;
use recording::Recording;

const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub fn launch(options: Options, program: Program) -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/ibniz.png"))
        .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("ibniz")
            .with_icon(icon)
            .with_inner_size([1080.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "IBNIZ Rust",
        native,
        Box::new(move |_cc| {
            let mut app = App::new(options, program);
            match nsm::Client::from_environment() {
                Ok(client) => app.nsm = client,
                Err(error) => app.nsm_error = Some(format!("NSM unavailable: {error}")),
            }
            Ok(Box::new(app))
        }),
    )
}

struct App {
    source: String,
    saved_source: String,
    compiled_source: String,
    path: String,
    vm: Vm,
    running: bool,
    editor: bool,
    help: bool,
    audio: Option<Audio>,
    dc_blocker: bool,
    volume: f32,
    texture: Option<TextureHandle>,
    error: Option<String>,
    compile_error: Option<String>,
    notice: String,
    budget: usize,
    render: Option<FrameRender>,
    render_work: Duration,
    slice_ms: u64,
    preview_fps: u32,
    next_preview: Instant,
    buffer_ms: u32,
    next_tick: Instant,
    last_instructions: usize,
    slow_video: bool,
    slow_audio: bool,
    late_ticks: u64,
    audio_budget_misses: u64,
    last_tick_duration: Duration,
    cursor: usize,
    recording: Option<Recording>,
    recording_directory: String,
    recording_mp4: bool,
    close_after_recording: bool,
    example_browser: examples::Browser,
    pending_example: Option<&'static examples::Example>,
    nsm: Option<nsm::Client>,
    session: Option<nsm::Project>,
    nsm_label: String,
    nsm_error: Option<String>,
    nsm_quitting: bool,
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
            compiled_source: options.source.clone(),
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
            dc_blocker: true,
            volume: 0.2,
            texture: None,
            error,
            compile_error: None,
            notice: String::new(),
            budget: options.budget,
            render: None,
            render_work: Duration::ZERO,
            slice_ms: 6,
            preview_fps: 60,
            next_preview: Instant::now(),
            buffer_ms: audio::DEFAULT_BUFFER_MS,
            next_tick: Instant::now(),
            last_instructions: 0,
            slow_video: false,
            slow_audio: false,
            late_ticks: 0,
            audio_budget_misses: 0,
            last_tick_duration: Duration::ZERO,
            cursor: 0,
            recording: None,
            recording_directory: "recordings".into(),
            recording_mp4: false,
            close_after_recording: false,
            example_browser: examples::Browser::default(),
            pending_example: None,
            nsm: None,
            session: None,
            nsm_label: String::new(),
            nsm_error: None,
            nsm_quitting: false,
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
        self.render = None;
        self.render_work = Duration::ZERO;
        self.next_tick = Instant::now();
        self.texture = None;
        if let Some(audio) = &mut self.audio {
            audio.clear();
        }
        self.reset_counters();
    }

    fn reset_counters(&mut self) {
        self.late_ticks = 0;
        self.audio_budget_misses = 0;
        self.last_tick_duration = Duration::ZERO;
        self.slow_video = false;
        self.slow_audio = false;
        if let Some(audio) = &mut self.audio {
            audio.reset_counters();
        }
    }

    fn recompile(&mut self) {
        match Program::compile(&self.source) {
            Ok(program) => {
                self.vm.replace_program(program);
                self.compiled_source = self.source.clone();
                self.compile_error = None;
            }
            Err(error) => self.compile_error = Some(error.to_string()),
        }
    }

    fn save(&mut self) {
        if self.managed() {
            if let Err(error) = self.save_session() {
                self.nsm_error = Some(error.message);
            }
            return;
        }
        match fs::write(&self.path, &self.source) {
            Ok(()) => {
                self.saved_source = self.source.clone();
                self.notice = format!("Saved {}", self.path);
            }
            Err(error) => self.error = Some(format!("Cannot save {}: {error}", self.path)),
        }
    }

    fn load(&mut self) {
        if self.managed() {
            self.error =
                Some("File Load is disabled under NSM; import a bundled example instead.".into());
            return;
        }
        match fs::read_to_string(&self.path) {
            Ok(source) => match Program::compile(&source) {
                Ok(program) => {
                    self.replace_source(source, program);
                    self.notice = format!("Loaded {}", self.path);
                }
                Err(error) => self.error = Some(format!("Cannot load {}: {error}", self.path)),
            },
            Err(error) => self.error = Some(format!("Cannot load {}: {error}", self.path)),
        }
    }

    fn replace_source(&mut self, source: String, program: Program) {
        self.source = source;
        self.saved_source = self.source.clone();
        self.compiled_source = self.source.clone();
        self.vm = Vm::new(program);
        self.render = None;
        self.render_work = Duration::ZERO;
        self.compile_error = None;
        self.next_tick = Instant::now();
        self.texture = None;
        self.cursor = 0;
        self.last_instructions = 0;
        if let Some(audio) = &mut self.audio {
            audio.clear();
        }
        self.reset_counters();
    }

    fn request_example(&mut self, example: &'static examples::Example) {
        if self.source != self.saved_source {
            self.pending_example = Some(example);
        } else {
            self.load_example(example);
        }
    }

    fn load_example(&mut self, example: &'static examples::Example) {
        if self.managed() && self.session.is_none() {
            self.nsm_error =
                Some("Wait for NSM to open a project before importing an example".into());
            return;
        }
        match Program::compile(example.source) {
            Ok(program) => {
                let previous_saved_source = self.saved_source.clone();
                self.replace_source(example.source.into(), program);
                if self.managed() {
                    self.saved_source = previous_saved_source;
                } else {
                    self.path = format!(
                        "{}-copy.ib",
                        example
                            .filename
                            .strip_suffix(".ib")
                            .expect("example filename"),
                    );
                }
                self.running = false;
                self.editor = true;
                self.example_browser.open = false;
                self.pending_example = None;
                self.notice = if self.managed() {
                    format!(
                        "Imported bundled {} into the NSM session (paused).",
                        example.title
                    )
                } else {
                    format!(
                        "Loaded bundled {} (paused). Save writes a copy to {}; change the path if needed.",
                        example.title, self.path,
                    )
                };
            }
            Err(error) => {
                self.pending_example = None;
                self.error = Some(format!("Cannot load bundled {}: {error}", example.filename));
            }
        }
    }

    fn confirm_example(&mut self, ctx: &egui::Context) {
        let Some(example) = self.pending_example else {
            return;
        };
        let mut discard = false;
        let mut cancel = false;
        let response =
            egui::Modal::new(egui::Id::new("replace-source-with-example")).show(ctx, |ui| {
                ui.heading("Replace unsaved edits?");
                ui.label(format!(
                    "Loading {} will replace your modified source and reset playback.",
                    example.title,
                ));
                ui.label("Cancel to save your work first.");
                ui.horizontal(|ui| {
                    discard = ui.button("Discard edits and load").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if discard {
            self.load_example(example);
        } else if cancel || response.should_close() {
            self.pending_example = None;
        }
    }

    fn managed(&self) -> bool {
        self.nsm.as_ref().is_some_and(nsm::Client::active)
    }

    fn session_state(&self) -> nsm::State {
        nsm::State {
            version: 1,
            source: self.source.clone(),
            compiled_source: self.compiled_source.clone(),
            running: self.running,
            editor: self.editor,
            dc_blocker: self.dc_blocker,
            volume: self.volume,
            budget: self.budget,
            slice_ms: self.slice_ms,
            preview_fps: self.preview_fps,
            buffer_ms: self.buffer_ms,
        }
    }

    fn modified(&self) -> bool {
        if self.managed() {
            self.session
                .as_ref()
                .is_some_and(|session| session.saved != self.session_state())
        } else {
            self.source != self.saved_source
        }
    }

    fn open_session(
        &mut self,
        ctx: &egui::Context,
        path: &str,
        display: &str,
        client_id: &str,
    ) -> Result<(), nsm::Failure> {
        if self.session.is_some() || self.recording.is_some() {
            return Err(nsm::Failure::new(
                nsm::NOT_NOW,
                "Session switching is not supported; restart this client",
            ));
        }
        if self.source != self.saved_source {
            return Err(nsm::Failure::new(
                nsm::UNSAVED_CHANGES,
                "Unsaved source edits would be lost",
            ));
        }
        if display.is_empty() || client_id.is_empty() {
            return Err(nsm::Failure::new(
                nsm::BAD_PROJECT,
                "NSM display name and client ID must not be empty",
            ));
        }
        let mut project = nsm::Project::open(Path::new(path), self.session_state())?;
        let state = project.saved.clone();
        let (program, compile_error, compiled_source) = match Program::compile(&state.source) {
            Ok(program) => (program, None, state.source.clone()),
            Err(error) => (
                Program::compile(&state.compiled_source)
                    .map_err(|e| nsm::Failure::new(nsm::BAD_PROJECT, e.to_string()))?,
                Some(error.to_string()),
                state.compiled_source.clone(),
            ),
        };
        self.replace_source(state.source, program);
        self.compiled_source = compiled_source;
        self.compile_error = compile_error;
        self.running = state.running;
        self.editor = state.editor;
        self.dc_blocker = state.dc_blocker;
        self.volume = state.volume;
        self.budget = state.budget;
        self.slice_ms = state.slice_ms;
        self.preview_fps = state.preview_fps;
        self.buffer_ms = state.buffer_ms;
        if let Some(audio) = &mut self.audio {
            audio.set_dc_blocker(self.dc_blocker);
            audio.set_buffer_ms(self.buffer_ms);
        }
        self.path = project
            .directory
            .join("state.json")
            .to_string_lossy()
            .into_owned();
        self.recording_directory = project
            .directory
            .join("recordings")
            .to_string_lossy()
            .into_owned();
        project.saved = self.session_state();
        self.session = Some(project);
        self.nsm_label = format!("{display} ({client_id})");
        self.pending_example = None;
        self.notice = format!("Opened NSM session {} at VM time zero", self.nsm_label);
        self.nsm_error = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "IBNIZ Rust - {}",
            self.nsm_label
        )));
        Ok(())
    }

    fn save_session(&mut self) -> Result<(), nsm::Failure> {
        if self.recording.is_some() {
            return Err(nsm::Failure::new(
                nsm::NOT_NOW,
                "Stop recording and wait for finalization before saving the NSM session",
            ));
        }
        let state = self.session_state();
        self.session
            .as_mut()
            .ok_or_else(|| nsm::Failure::new(nsm::NO_SESSION, "NSM has not opened a project yet"))?
            .save(state)?;
        self.saved_source = self.source.clone();
        self.notice = format!("Saved NSM session {}", self.nsm_label);
        self.nsm_error = None;
        Ok(())
    }

    fn poll_nsm(&mut self, ctx: &egui::Context) {
        let Some(client) = &mut self.nsm else {
            return;
        };
        if client.termination_requested() {
            self.nsm_quitting = true;
            self.running = false;
            self.pending_example = None;
            if let Some(audio) = &mut self.audio {
                audio.clear();
            }
            if let Some(recording) = &mut self.recording {
                recording.request_shutdown();
            }
            return;
        }
        let events = client.poll();
        for event in events {
            let (path, result) = match event {
                nsm::Event::Announced => {
                    self.notice = "NSM connected; waiting for a project".into();
                    continue;
                }
                nsm::Event::Open {
                    path,
                    display,
                    client_id,
                } => (
                    nsm::OPEN,
                    self.open_session(ctx, &path, &display, &client_id),
                ),
                nsm::Event::Save => (nsm::SAVE, self.save_session()),
                nsm::Event::Warning(error) => {
                    eprintln!("ibniz: {error}");
                    self.nsm_error = Some(error);
                    continue;
                }
            };
            if let Err(error) = &result {
                self.nsm_error = Some(error.message.clone());
            }
            if let Err(error) = self.nsm.as_ref().expect("NSM client").reply(path, result) {
                eprintln!("ibniz: {error}");
                self.nsm_error = Some(error);
            }
        }
    }

    fn report_nsm_dirty(&mut self) {
        if self.session.is_some() {
            let dirty = self.modified();
            if let Some(client) = &mut self.nsm
                && let Err(error) = client.report_dirty(dirty)
            {
                eprintln!("ibniz: {error}");
                self.nsm_error = Some(error);
            }
        }
    }

    fn start_recording(&mut self) {
        if self.managed() && self.session.is_none() {
            self.nsm_error = Some("Wait for NSM to open a project before recording".into());
            return;
        }
        if self.recording.is_some() {
            self.error = Some("A recording is already active or finishing".into());
            return;
        }
        match Recording::start(Path::new(&self.recording_directory), self.recording_mp4) {
            Ok(recording) => {
                self.notice = format!("Recording to {}", recording.directory.display());
                self.recording = Some(recording);
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn stop_recording(&mut self) {
        if let Some(recording) = &mut self.recording {
            recording.stop();
        }
    }

    fn poll_recording(&mut self, ctx: &egui::Context) {
        let Some(result) = self.recording.as_mut().and_then(Recording::poll) else {
            return;
        };
        self.recording = None;
        match result {
            Ok(completed) => {
                self.notice = format!(
                    "Saved {} tick(s) ({:.2} s) to {}{}",
                    completed.frames,
                    completed.frames as f64 / 60.0,
                    completed.directory.display(),
                    if completed.mp4 {
                        " (PPM + WAV + recording.mp4)"
                    } else if completed.frames == 0 {
                        " (empty WAV; no frames or MP4)"
                    } else {
                        " (PPM + WAV)"
                    },
                );
                if self.close_after_recording {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Err(error) => {
                self.notice.clear();
                self.error = Some(error);
                self.close_after_recording = false;
            }
        }
    }

    fn request_close(&mut self, ctx: &egui::Context) {
        if self.recording.is_some() {
            self.close_after_recording = true;
            self.stop_recording();
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending_example.is_some() {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.pending_example = None;
            }
            return;
        }
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
            } else if self.example_browser.open {
                self.example_browser.open = false;
            } else {
                self.request_close(ctx);
            }
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        if let Some(audio) = &mut self.audio {
            audio.set_playing(
                self.running && self.compile_error.is_none() && self.vm.status().audio_enabled,
            );
        }
        if self.running && self.compile_error.is_none() && Instant::now() >= self.next_tick {
            if self.render.is_none() {
                self.render = Some(self.vm.begin_frame(self.budget));
                self.render_work = Duration::ZERO;
            }
            let started = Instant::now();
            let frame = loop {
                let frame = self
                    .render
                    .as_mut()
                    .expect("active frame")
                    .advance(&mut self.vm, 1024);
                if frame.is_some() || started.elapsed() >= Duration::from_millis(self.slice_ms) {
                    break frame;
                }
            };
            self.render_work += started.elapsed();
            let Some(frame) = frame else {
                ctx.request_repaint();
                return;
            };
            self.render = None;
            let finishing = Instant::now();
            self.last_instructions = frame.instructions;
            self.slow_video = !frame.video_complete && !self.vm.status().stopped;
            self.slow_audio = !frame.audio_complete
                && self.vm.status().audio_enabled
                && !self.vm.status().stopped;
            if self.slow_audio {
                self.audio_budget_misses += 1;
            }
            let preview_now = Instant::now();
            if self.preview_fps == 60 || self.texture.is_none() || preview_now >= self.next_preview
            {
                let image = egui::ColorImage::from_rgb([WIDTH, HEIGHT], &frame.rgb);
                if let Some(texture) = &mut self.texture {
                    texture.set(image, egui::TextureOptions::NEAREST);
                } else {
                    self.texture =
                        Some(ctx.load_texture("ibniz-video", image, egui::TextureOptions::NEAREST));
                }
                let interval = Duration::from_secs_f64(1.0 / f64::from(self.preview_fps));
                self.next_preview += interval;
                if self.next_preview < preview_now {
                    self.next_preview = preview_now + interval;
                }
            }
            if let Some(audio) = &mut self.audio {
                audio.enqueue(&frame.audio, self.volume);
            }
            if let Some(recording) = &mut self.recording
                && recording.is_capturing()
                && let Err(error) = recording.capture(&frame)
            {
                self.error = Some(error);
            }
            if self.vm.status().stopped {
                self.running = false;
                self.stop_recording();
                if let Some(audio) = &mut self.audio {
                    audio.set_playing(false);
                }
            }
            self.last_tick_duration = self.render_work + finishing.elapsed();
            self.next_tick += TICK_DURATION;
            // Don't replay an unbounded backlog after dragging or suspending the window.
            if self.next_tick < Instant::now() {
                self.late_ticks += 1;
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
        let keyboard_to_vm = !self.editor && !ui.ctx().wants_keyboard_input();
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
            if keyboard_to_vm {
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
        self.poll_nsm(ctx);
        self.poll_recording(ctx);
        if self.nsm_quitting {
            if self.recording.is_none() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(Duration::from_millis(10));
            }
            return;
        }
        self.shortcuts(ctx);
        if ctx.input(|i| i.viewport().close_requested()) {
            self.request_close(ctx);
        }
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
                if ui.button("Examples").clicked() {
                    self.example_browser.open = !self.example_browser.open;
                }
                ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume"));
                ui.menu_button("Audio", |ui| {
                    if ui
                        .checkbox(&mut self.dc_blocker, "DC blocker")
                        .on_hover_text(
                            "Remove DC offset with a 20 Hz high-pass filter (playback only).",
                        )
                        .changed()
                        && let Some(audio) = &mut self.audio
                    {
                        audio.set_dc_blocker(self.dc_blocker);
                    }
                });
                ui.menu_button("Playback", |ui| {
                    ui.label("Preview refresh (display only)");
                    ui.horizontal(|ui| {
                        for fps in [15, 30, 60] {
                            ui.selectable_value(&mut self.preview_fps, fps, format!("{fps} fps"));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Instructions/tick");
                        ui.add(
                            egui::DragValue::new(&mut self.budget)
                                .range(4..=usize::MAX)
                                .speed(10_000),
                        ).on_hover_text(
                            "Applied to the next VM tick. Higher budgets allow more complex \
                             expressions but may slow playback. No VM reset.",
                        );
                    });
                    ui.add(egui::Slider::new(&mut self.slice_ms, 1..=12).text("Work slice (ms)"))
                        .on_hover_text(
                            "Maximum VM work per UI update, checked every 1024 instructions. \
                             Lower values favor editor responsiveness; higher values favor throughput. \
                             Finishing a tick also converts its video image.",
                        );
                    if ui.add(
                        egui::Slider::new(&mut self.buffer_ms, 20..=200)
                            .text("Minimum audio buffer (ms)"),
                    ).on_hover_text(
                        "More buffering tolerates scheduling jitter but adds latency. \
                         The target grows automatically after underruns, up to 200 ms.",
                    ).changed() && let Some(audio) = &mut self.audio {
                        audio.set_buffer_ms(self.buffer_ms);
                    }
                    ui.small("Preview refresh does not change VM or 60 fps recording quality.");
                });
            });
            if self.editor {
                ui.horizontal(|ui| {
                    ui.label("File");
                    ui.add_enabled(!self.managed(), egui::TextEdit::singleline(&mut self.path));
                    if ui.add_enabled(!self.managed(), egui::Button::new("Load")).clicked() {
                        self.load();
                    }
                    if ui.button(if self.managed() { "Save session" } else { "Save" }).clicked() {
                        self.save();
                    }
                    if self.modified() {
                        ui.label("Modified");
                    }
                });
            }
            ui.collapsing("Recording", |ui| {
                ui.add_enabled_ui(self.recording.is_none(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Output folder");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recording_directory)
                                .desired_width(250.0),
                        );
                        ui.checkbox(&mut self.recording_mp4, "Create MP4")
                            .on_hover_text(
                                "After Stop, use installed FFmpeg to encode H.264/AAC. \
                                 PPM frames and WAV are retained. No automatic downloads.",
                            );
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    if self.recording.is_none() {
                        if ui
                            .add_enabled(
                                self.compile_error.is_none() && !self.vm.status().stopped,
                                egui::Button::new("Start recording"),
                            )
                            .on_hover_text(
                                "Capture future VM ticks without resetting playback. \
                                 If paused, press Run to begin capture. Reset a terminated VM first.",
                            )
                            .clicked()
                        {
                            self.start_recording();
                        }
                    } else if self.recording.as_ref().is_some_and(Recording::is_capturing) {
                        if ui.button("Stop recording").clicked() {
                            self.stop_recording();
                        }
                    } else {
                        ui.spinner();
                        ui.label("Finalizing WAV / encoding MP4...");
                    }
                    ui.small("Each take gets a new subfolder. Audio is full-scale, unfiltered VM output.");
                });
            });
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
                    "Instruction budget reached: video={}, audio={}. Raise Playback > Instructions/tick for complex programs.",
                    if self.slow_video { "behind" } else { "ready" },
                    if self.slow_audio { "behind" } else { "ready" },
                ));
            }
            if let Some(render) = &self.render {
                ui.small(format!(
                    "Computing tick: {} / {} instructions ({} ms work slices)",
                    render.instructions(), render.budget(), self.slice_ms,
                ));
            }
            ui.label(format!(
                "Late ticks: {} | Last tick: {:.1} ms / 16.7 ms | Audio budget misses: {}",
                self.late_ticks, self.last_tick_duration.as_secs_f64() * 1000.0,
                self.audio_budget_misses,
            )).on_hover_text(
                "Late ticks missed the 60 Hz deadline (rendering cost or UI/scheduling delays). \
                 Audio budget misses mean the VM could not generate a full audio tick. \
                 These are not necessarily device playback gaps. F2 or loading a file resets counters.",
            );
            if let Some(error) = &self.compile_error {
                ui.colored_label(Color32::RED, format!("Compile error: {error}. Playback suspended until corrected."));
            }
            if let Some(audio) = &self.audio {
                let status = audio.status();
                if let Some(error) = status.error { ui.colored_label(Color32::RED, format!("Audio error: {error}")); }
                let color = if status.underruns > 0 || status.dropped > 0 {
                    Color32::YELLOW
                } else {
                    ui.visuals().text_color()
                };
                ui.colored_label(color, format!(
                    "Audio gaps: {} ({:.1} ms missing) | Discarded queued samples: {}",
                    status.underruns, status.gap_ms, status.dropped,
                )).on_hover_text(
                    "Gaps count output callbacks without ready audio during playback, including \
                     recovery buffering; \
                     one audible interruption may span several callbacks. Startup, pauses and \
                     compile-error suspension do not count. Discarded samples indicate queue \
                     overflow. This measures buffer starvation, not clicks in the expression \
                     itself or every possible device glitch. F2 or loading a file resets counters.",
                );
                ui.small(format!(
                    "Audio buffer: {:.0} / {:.0} ms{}",
                    status.buffered_ms, status.target_ms,
                    if status.buffering { " (buffering)" } else { "" },
                ));
            }
            if let Some(error) = &self.error {
                ui.colored_label(Color32::RED, error);
                if ui.small_button("Dismiss").clicked() { self.error = None; }
            }
            if let Some(client) = &self.nsm {
                if client.active() {
                    ui.small(format!("NSM: {} | {}", client.manager,
                        if self.session.is_some() { self.nsm_label.as_str() } else { "waiting for project" }));
                } else if client.needs_poll() {
                    ui.small("NSM: waiting for announce reply");
                }
            }
            if let Some(error) = &self.nsm_error {
                ui.colored_label(Color32::RED, error);
                if ui.small_button("Dismiss NSM error").clicked() { self.nsm_error = None; }
            }
            if let Some(recording) = &self.recording {
                ui.colored_label(
                    if recording.is_capturing() { Color32::RED } else { Color32::YELLOW },
                    format!(
                        "{}: {} tick(s), {:.2} s | {}",
                        if recording.is_capturing() {
                            if self.running && self.compile_error.is_none() { "Recording" } else { "Recording paused" }
                        } else { "Finalizing recording" },
                        recording.frames, recording.frames as f64 / 60.0,
                        recording.directory.display(),
                    ),
                );
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
        let managed = self.managed();
        if let Some(example) = self.example_browser.show(ctx, managed) {
            self.request_example(example);
        }
        egui::Window::new("IBNIZ Help (F12)")
            .open(&mut self.help)
            .default_width(520.0)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.heading("Keyboard shortcuts");
                ui.label("F12: show / hide help\nEsc: cancel example loading, close help or examples, otherwise exit\nF1: run / pause\nF2: reset memory, stacks and time\nTab: show / hide editor\nCtrl/Cmd+S: save\nCtrl+Up / Down: adjust the hex digit under the cursor");
                ui.label("Use platform-standard shortcuts for clipboard, selection, undo/redo and word navigation.");
                ui.label("Examples: search bundled programs by title, description, contributor or source, and filter by category. Loading resets and pauses playback, asks before discarding edits, and suggests a local copy path outside NSM sessions. Credits, pinned source links and collection licenses are available offline (links require a browser).");
                ui.label("NSM: launch from a session manager to save source and playback settings in its assigned project. Loading restarts the VM at time zero. File Load is disabled; examples are imported into the session. Stop recording and wait for finalization before saving.");
                ui.label("Audio > DC blocker: remove DC offset from speaker playback (enabled by default). Headless WAV exports use the separate --dc-blocker flag.");
                ui.label("Recording: choose an output folder and Start/Stop to save PPM frames and full-scale, unfiltered WAV audio. Run/pause and compile errors pause capture. Optional MP4 uses installed FFmpeg after Stop; original files are retained.");
                ui.label("Playback: adjust preview refresh, instructions per VM tick, UI work slices and audio buffering without resetting the VM. Preview refresh affects only the display, not recordings. Audio rebuffering uses short fades; sustained overload can still cause gaps.");
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
        self.confirm_example(ctx);
        self.tick(ctx);
        self.report_nsm_dirty();
        if self.recording.is_some() || self.nsm.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
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
                wav_dc_blocker: false,
                budget: 1_000_000,
            },
            Program::compile("^xp M d3r15&*").unwrap(),
        )
    }

    fn complete_tick(app: &mut App, ctx: &egui::Context) {
        let target = app.vm.status().frame.wrapping_add(1);
        let deadline = Instant::now() + Duration::from_secs(10);
        app.next_tick = Instant::now();
        while app.vm.status().frame != target {
            assert!(Instant::now() < deadline, "VM tick did not complete");
            app.tick(ctx);
        }
    }

    fn bundled_example(filename: &str) -> &'static examples::Example {
        examples::EXAMPLES
            .iter()
            .find(|example| example.filename == filename)
            .unwrap()
    }

    fn click_label(app: &mut App, ctx: &egui::Context, label: &str) {
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
        let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
        let position = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing UI label: {label}"));
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(position)],
                ..Default::default()
            },
            |ctx| app.ui(ctx),
        );
        for pressed in [true, false] {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
        }
    }

    fn managed_app() -> (App, nsm::tests::Server) {
        let (server, mut client) = nsm::tests::Server::pair();
        server.accept(&mut client);
        let mut app = app();
        app.nsm = Some(client);
        (app, server)
    }

    fn open_managed(app: &mut App, server: &nsm::tests::Server, ctx: &egui::Context, path: &Path) {
        server.open(path);
        let deadline = Instant::now() + Duration::from_secs(2);
        while app.session.is_none() && app.nsm_error.is_none() {
            assert!(Instant::now() < deadline, "NSM open was not processed");
            app.poll_nsm(ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        let reply = server.receive();
        assert_eq!(reply.addr, "/reply", "{reply:?}");
        assert_eq!(reply.args[0], rosc::OscType::String(nsm::OPEN.into()));
    }

    #[test]
    fn nsm_open_save_dirty_and_restore_preserve_source_and_settings_but_reset_vm() {
        let directory = nsm::tests::TestDirectory::new();
        let path = directory.0.join("IBNIZ session.nTEST");
        let ctx = egui::Context::default();
        let (mut app, server) = managed_app();
        open_managed(&mut app, &server, &ctx, &path);
        assert!(app.managed());
        assert!(!app.modified());
        app.report_nsm_dirty();
        assert_eq!(server.receive().addr, "/nsm/client/is_clean");
        app.volume = 0.45;
        app.preview_fps = 15;
        app.budget = 2_000_000;
        app.buffer_ms = 140;
        app.dc_blocker = false;
        app.slice_ms = 3;
        app.editor = false;
        app.running = true;
        app.vm.memory_mut()[0x123] = 42;
        app.report_nsm_dirty();
        assert_eq!(server.receive().addr, "/nsm/client/is_dirty");
        app.source = "invalid".into();
        app.recompile();
        assert!(app.compile_error.is_some());
        server.send(nsm::SAVE, vec![]);
        assert_eq!(server.receive_after(|| app.poll_nsm(&ctx)).addr, "/reply");
        assert!(!app.modified());
        app.report_nsm_dirty();
        assert_eq!(server.receive().addr, "/nsm/client/is_clean");

        let (mut restored, restored_server) = managed_app();
        open_managed(&mut restored, &restored_server, &ctx, &path);
        assert_eq!(restored.source, "invalid");
        assert!(restored.compile_error.is_some());
        assert_eq!(restored.compiled_source, "^xp M d3r15&*");
        assert_eq!(restored.volume, 0.45);
        assert_eq!(restored.preview_fps, 15);
        assert_eq!(restored.budget, 2_000_000);
        assert_eq!(restored.buffer_ms, 140);
        assert!(!restored.dc_blocker);
        assert_eq!(restored.slice_ms, 3);
        assert!(!restored.editor);
        assert!(restored.running);
        assert_eq!(restored.vm.status().frame, 0);
        assert_eq!(restored.vm.memory()[0x123], 0);
        assert_eq!(
            Path::new(&restored.recording_directory),
            path.join("recordings")
        );
        let managed_path = restored.path.clone();
        restored.request_example(bundled_example("plasma.ib"));
        assert_eq!(restored.source, bundled_example("plasma.ib").source);
        assert_eq!(restored.path, managed_path);
        assert!(!restored.running);
        assert!(restored.modified());
        restored.save();
        let saved: nsm::State =
            serde_json::from_slice(&fs::read(path.join("state.json")).unwrap()).unwrap();
        assert_eq!(saved.source, bundled_example("plasma.ib").source);
        assert!(!restored.modified());
        restored.path = directory
            .0
            .join("must-not-create.ib")
            .to_string_lossy()
            .into_owned();
        restored.load();
        assert_eq!(restored.source, saved.source);
        restored.save();
        assert!(!Path::new(&restored.path).exists());
        assert!(path.join("state.json").exists());
    }

    #[test]
    fn nsm_open_errors_preserve_edits_and_reject_session_switching() {
        let directory = nsm::tests::TestDirectory::new();
        let ctx = egui::Context::default();
        let (mut app, server) = managed_app();
        app.source.push_str(" 1");
        server.open(&directory.0.join("first"));
        let error = server.receive_after(|| app.poll_nsm(&ctx));
        assert_eq!(error.addr, "/error");
        assert_eq!(error.args[1], rosc::OscType::Int(nsm::UNSAVED_CHANGES));
        assert!(app.session.is_none());
        assert!(app.source.ends_with(" 1"));
        app.source = app.saved_source.clone();
        app.nsm_error = None;
        open_managed(&mut app, &server, &ctx, &directory.0.join("first"));
        server.open(&directory.0.join("second"));
        let error = server.receive_after(|| app.poll_nsm(&ctx));
        assert_eq!(error.addr, "/error");
        assert_eq!(error.args[1], rosc::OscType::Int(nsm::NOT_NOW));
        assert!(!directory.0.join("second").exists());
    }

    #[test]
    fn nsm_save_reports_not_now_while_recording_and_sigterm_finalizes_without_dialogs() {
        let directory = nsm::tests::TestDirectory::new();
        let ctx = egui::Context::default();
        let (mut app, server) = managed_app();
        open_managed(&mut app, &server, &ctx, &directory.0.join("client.nTEST"));
        app.start_recording();
        let frame = app.vm.render_frame(app.budget);
        app.recording.as_mut().unwrap().capture(&frame).unwrap();
        let take = app.recording.as_ref().unwrap().directory.clone();
        server.send(nsm::SAVE, vec![]);
        let error = server.receive_after(|| app.poll_nsm(&ctx));
        assert_eq!(error.addr, "/error");
        assert_eq!(error.args[1], rosc::OscType::Int(nsm::NOT_NOW));
        app.source.push_str(" 1");
        app.request_example(bundled_example("plasma.ib"));
        assert!(app.pending_example.is_some());
        app.nsm.as_ref().unwrap().request_termination();
        let deadline = Instant::now() + Duration::from_secs(5);
        let closed = loop {
            let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
            if output
                .viewport_output
                .values()
                .flat_map(|output| &output.commands)
                .any(|command| matches!(command, egui::ViewportCommand::Close))
            {
                break true;
            }
            assert!(Instant::now() < deadline, "NSM shutdown did not finish");
            std::thread::sleep(Duration::from_millis(1));
        };
        assert!(closed);
        assert!(app.nsm_quitting);
        assert!(!app.running);
        assert!(app.pending_example.is_none());
        assert!(app.recording.is_none());
        assert_eq!(
            fs::read(take.join("sound.wav")).unwrap().len(),
            44 + 1024 * 2
        );
        assert!(take.join("frame-000000.ppm").exists());
    }

    #[test]
    fn loading_bundled_examples_resets_and_pauses_without_changing_playback_settings() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.slice_ms = 0;
        app.toggle_running();
        app.tick(&ctx);
        assert!(app.render.is_some());
        app.vm.memory_mut()[0x123] = 42;
        app.late_ticks = 10;
        app.audio_budget_misses = 12;
        app.cursor = 5;
        app.editor = false;
        let budget = app.budget;
        app.request_example(bundled_example("plasma.ib"));
        assert_eq!(app.source, bundled_example("plasma.ib").source);
        assert_eq!(app.source, app.saved_source);
        assert_eq!(app.path, "plasma-copy.ib");
        assert!(!app.running);
        assert!(app.editor);
        assert!(app.render.is_none());
        assert!(app.texture.is_none());
        assert!(app.compile_error.is_none());
        assert!(app.pending_example.is_none());
        assert_eq!(app.vm.memory()[0x123], 0);
        assert_eq!(app.vm.status().frame, 0);
        assert_eq!(app.late_ticks, 0);
        assert_eq!(app.audio_budget_misses, 0);
        assert_eq!(app.cursor, 0);
        assert_eq!(app.budget, budget);
        assert_eq!(app.slice_ms, 0);
        app.toggle_running();
        complete_tick(&mut app, &ctx);
        assert_eq!(app.vm.status().frame, 1);
    }

    #[test]
    fn example_load_confirmation_can_cancel_or_discard_unsaved_edits() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.example_browser.open = true;
        app.source.push_str(" 1");
        let edited = app.source.clone();
        app.vm.memory_mut()[0x123] = 42;
        app.request_example(bundled_example("glass.ib"));
        assert!(app.pending_example.is_some());
        assert_eq!(app.source, edited);
        click_label(&mut app, &ctx, "Cancel");
        assert!(app.pending_example.is_none());
        assert_eq!(app.source, edited);
        assert_eq!(app.vm.memory()[0x123], 42);
        assert!(app.example_browser.open);

        app.request_example(bundled_example("glass.ib"));
        click_label(&mut app, &ctx, "Discard edits and load");
        assert!(app.pending_example.is_none());
        assert_eq!(app.source, bundled_example("glass.ib").source);
        assert_eq!(app.path, "glass-copy.ib");
        assert_eq!(app.vm.memory()[0x123], 0);
        assert!(!app.running);
        assert!(!app.example_browser.open);
    }

    #[test]
    fn escape_cancels_example_loading_then_closes_browser_before_exiting() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.example_browser.open = true;
        app.source.push_str(" 1");
        let edited = app.source.clone();
        app.request_example(bundled_example("glass.ib"));
        for (browser_open, closing) in [(true, false), (false, false), (false, true)] {
            let output = ctx.run(
                egui::RawInput {
                    events: [true, false]
                        .into_iter()
                        .map(|pressed| egui::Event::Key {
                            key: Key::Escape,
                            physical_key: None,
                            pressed,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        })
                        .collect(),
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
            assert!(app.pending_example.is_none());
            assert_eq!(app.example_browser.open, browser_open);
            assert_eq!(app.source, edited);
            assert_eq!(
                output
                    .viewport_output
                    .values()
                    .flat_map(|output| &output.commands)
                    .any(|command| matches!(command, egui::ViewportCommand::Close)),
                closing,
            );
        }
    }

    #[test]
    fn examples_button_search_and_load_work_without_sending_search_text_to_vm() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.editor = false;
        click_label(&mut app, &ctx, "Examples");
        assert!(app.example_browser.open);
        click_label(
            &mut app,
            &ctx,
            "Title, filename, description, contributor or source",
        );
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Text("plasma".into())],
                ..Default::default()
            },
            |ctx| app.ui(ctx),
        );
        assert_eq!((app.vm.input() >> 16) & 255, 0);
        let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
        for label in ["1 / 38 examples", "Plasma", "IBNIZ language manual"] {
            assert!(output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text() == label)
            }), "missing browser content: {label}");
        }
        click_label(&mut app, &ctx, "Load example");
        assert_eq!(app.source, bundled_example("plasma.ib").source);
        assert!(!app.running);
        assert!(!app.example_browser.open);
    }

    #[test]
    fn embedded_application_icon_decodes_to_rgba() {
        let icon =
            eframe::icon_data::from_png_bytes(include_bytes!("../../assets/ibniz.png")).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        assert!(
            icon.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[1] > 0 && pixel[3] > 0)
        );
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
        if app.vm.status().frame == 0 {
            complete_tick(&mut app, &ctx);
        }
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
    fn playback_diagnostics_count_late_and_budget_limited_ticks_and_reset() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.budget = 1;
        app.toggle_running();
        app.next_tick = Instant::now() - Duration::from_secs(1);
        app.tick(&ctx);
        assert_eq!(app.late_ticks, 1);
        assert_eq!(app.audio_budget_misses, 1);
        assert!(app.slow_audio);

        app.toggle_running();
        app.tick(&ctx);
        assert_eq!(app.late_ticks, 1);
        assert_eq!(app.audio_budget_misses, 1);
        app.source.push(' ');
        app.recompile();
        assert_eq!(app.audio_budget_misses, 1);

        app.toggle_running();
        app.source = "invalid".into();
        app.recompile();
        app.tick(&ctx);
        assert_eq!(app.audio_budget_misses, 1);

        app.reset();
        assert_eq!(app.late_ticks, 0);
        assert_eq!(app.audio_budget_misses, 0);
        assert_eq!(app.last_tick_duration, Duration::ZERO);
        assert!(!app.slow_audio);
        assert!(!app.slow_video);
    }

    #[test]
    fn gui_recording_captures_vm_ticks_pauses_and_finalizes_before_close() {
        let ctx = egui::Context::default();
        let mut app = app();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let parent = std::env::temp_dir().join(format!(
            "ibniz-gui-recording-{timestamp}-{}",
            std::process::id(),
        ));
        app.recording_directory = parent.to_string_lossy().into_owned();
        app.volume = 0.0;
        app.preview_fps = 15;
        app.start_recording();
        let directory = app.recording.as_ref().unwrap().directory.clone();
        app.tick(&ctx);
        assert_eq!(app.recording.as_ref().unwrap().frames, 0);

        app.toggle_running();
        for _ in 0..2 {
            complete_tick(&mut app, &ctx);
        }
        assert_eq!(app.recording.as_ref().unwrap().frames, 2);
        app.toggle_running();
        app.tick(&ctx);
        assert_eq!(app.recording.as_ref().unwrap().frames, 2);
        app.toggle_running();
        app.source = "invalid".into();
        app.recompile();
        app.tick(&ctx);
        assert_eq!(app.recording.as_ref().unwrap().frames, 2);
        app.source = "^xp M d3r15&*".into();
        app.recompile();
        complete_tick(&mut app, &ctx);
        assert_eq!(app.recording.as_ref().unwrap().frames, 3);

        app.request_close(&ctx);
        assert!(app.close_after_recording);
        assert!(!app.recording.as_ref().unwrap().is_capturing());
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.recording.is_some() {
            assert!(Instant::now() < deadline, "recording did not finalize");
            app.poll_recording(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.error.is_none(), "{:?}", app.error);
        assert!(app.notice.contains("Saved 3 tick(s)"));
        let wav = fs::read(directory.join("sound.wav")).unwrap();
        let mut vm = Vm::new(Program::compile("^xp M d3r15&*").unwrap());
        let samples: Vec<_> = (0..3)
            .flat_map(|_| vm.render_frame(1_000_000).audio)
            .collect();
        let mut expected = Vec::new();
        ibniz::export::write_wav(&mut expected, &samples).unwrap();
        assert_eq!(wav, expected);
        assert!(directory.join("frame-000002.ppm").exists());
        assert!(!directory.join("frame-000003.ppm").exists());
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn expensive_ticks_yield_and_resume_without_resetting_or_changing_their_budget() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.vm = Vm::new(Program::compile("[1]").unwrap());
        app.budget = 50_000;
        app.slice_ms = 0;
        app.toggle_running();
        app.tick(&ctx);
        assert_eq!(app.vm.status().frame, 0);
        assert_eq!(app.render.as_ref().unwrap().instructions(), 1024);
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::F12,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.ui(ctx),
        );
        assert!(app.help);
        assert!(app.render.as_ref().unwrap().instructions() <= 2048);
        app.toggle_running();
        let instructions = app.render.as_ref().unwrap().instructions();
        app.tick(&ctx);
        assert_eq!(app.render.as_ref().unwrap().instructions(), instructions);
        app.budget = 64;
        app.toggle_running();
        complete_tick(&mut app, &ctx);
        assert_eq!(app.last_instructions, 50_000);
        complete_tick(&mut app, &ctx);
        assert!(app.last_instructions <= 64);
        app.next_tick = Instant::now();
        app.budget = 50_000;
        app.tick(&ctx);
        assert!(app.render.is_some());
        app.reset();
        assert!(app.render.is_none());
        assert_eq!(app.vm.status().frame, 0);
    }

    #[test]
    fn preview_refresh_only_throttles_texture_uploads_not_vm_ticks() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.preview_fps = 15;
        app.toggle_running();
        let first = ctx.run(egui::RawInput::default(), |ctx| {
            complete_tick(&mut app, ctx);
        });
        let texture = app.texture.as_ref().unwrap().id();
        assert!(
            first
                .textures_delta
                .set
                .iter()
                .any(|(id, _)| *id == texture)
        );
        app.next_preview = Instant::now() + Duration::from_secs(60);
        let second = ctx.run(egui::RawInput::default(), |ctx| {
            complete_tick(&mut app, ctx);
        });
        assert!(
            !second
                .textures_delta
                .set
                .iter()
                .any(|(id, _)| *id == texture)
        );
        assert_eq!(app.vm.status().frame, 2);
        app.preview_fps = 60;
        let third = ctx.run(egui::RawInput::default(), |ctx| {
            complete_tick(&mut app, ctx);
        });
        assert!(
            third
                .textures_delta
                .set
                .iter()
                .any(|(id, _)| *id == texture)
        );
        assert_eq!(app.vm.status().frame, 3);
    }

    #[test]
    fn playback_menu_changes_preview_quality_without_resetting_vm_or_budget() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.vm.memory_mut()[0x123] = 42;
        let budget = app.budget;
        for (label, fps) in [
            ("Playback", 60),
            ("15 fps", 15),
            ("Playback", 15),
            ("30 fps", 30),
            ("Playback", 30),
            ("60 fps", 60),
        ] {
            let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
            let position = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing playback control: {label}"));
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(position)],
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
            for pressed in [true, false] {
                let _ = ctx.run(
                    egui::RawInput {
                        events: vec![egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        }],
                        ..Default::default()
                    },
                    |ctx| app.ui(ctx),
                );
            }
            assert_eq!(app.preview_fps, fps);
            assert_eq!(app.vm.memory()[0x123], 42);
            assert_eq!(app.vm.status().frame, 0);
            assert_eq!(app.budget, budget);
            assert!(!app.running);
        }
    }

    #[test]
    fn recording_panel_buttons_and_mp4_option_work_without_an_audio_device() {
        let ctx = egui::Context::default();
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut app = app();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let parent = std::env::temp_dir().join(format!(
            "ibniz-recording-controls-{timestamp}-{}",
            std::process::id(),
        ));
        app.recording_directory = parent.to_string_lossy().into_owned();
        for label in [
            "Recording",
            "Create MP4",
            "Create MP4",
            "Start recording",
            "Stop recording",
        ] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
            let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
            let position = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing recording control: {label}"));
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(position)],
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
            for pressed in [true, false] {
                let _ = ctx.run(
                    egui::RawInput {
                        events: vec![egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        }],
                        ..Default::default()
                    },
                    |ctx| app.ui(ctx),
                );
            }
            if label == "Start recording" {
                assert!(app.recording.as_ref().unwrap().is_capturing());
                assert!(!app.running);
            }
        }
        assert!(!app.recording_mp4);
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.recording.is_some() {
            assert!(Instant::now() < deadline, "recording did not finalize");
            app.poll_recording(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.error.is_none(), "{:?}", app.error);
        assert!(app.notice.contains("empty WAV"));
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn audio_menu_toggles_dc_blocker_without_changing_vm_state() {
        let ctx = egui::Context::default();
        let mut app = app();
        assert!(app.dc_blocker);
        app.vm.memory_mut()[0x123] = 42;
        let source = app.source.clone();
        for (label, enabled) in [
            ("Audio", true),
            ("DC blocker", false),
            ("Audio", false),
            ("DC blocker", true),
        ] {
            let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
            let position = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    panic!("missing menu label: {label}, expected enabled={enabled}")
                });
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(position)],
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
            for pressed in [true, false] {
                let _ = ctx.run(
                    egui::RawInput {
                        events: vec![egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        }],
                        ..Default::default()
                    },
                    |ctx| app.ui(ctx),
                );
            }
            assert_eq!(app.dc_blocker, enabled);
            assert_eq!(app.source, source);
            assert_eq!(app.vm.status().frame, 0);
            assert_eq!(app.vm.memory()[0x123], 42);
            assert!(!app.running);
        }
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
