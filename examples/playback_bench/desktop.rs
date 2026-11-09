use super::{Runner, Upload, UploadTiming};
use eframe::{
    egui,
    glow::{self, HasContext},
};
use std::{
    error::Error,
    io,
    sync::{Arc, Mutex},
    time::Instant,
};

struct TextureUpload {
    gl: Arc<glow::Context>,
    texture: glow::Texture,
}

impl TextureUpload {
    fn new(gl: Arc<glow::Context>) -> Result<Self, Box<dyn Error>> {
        // The native app invokes this and all uploads on the current GL thread.
        let texture = unsafe { gl.create_texture().map_err(io::Error::other)? };
        Ok(Self { gl, texture })
    }
}

impl Upload for TextureUpload {
    fn upload(&mut self, rgb: &[u8]) -> Result<UploadTiming, Box<dyn Error>> {
        let started = Instant::now();
        let image = egui::ColorImage::from_rgb([ibniz::WIDTH, ibniz::HEIGHT], rgb);
        let rgba: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect();
        let prepare = started.elapsed();
        let upload;
        unsafe {
            let previous = self.gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            // Exclude earlier egui work; include completion, not just command queueing.
            self.gl.finish();
            let started = Instant::now();
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                ibniz::WIDTH as i32,
                ibniz::HEIGHT as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&rgba)),
            );
            self.gl.finish();
            upload = started.elapsed();
            let error = self.gl.get_error();
            self.gl.bind_texture(glow::TEXTURE_2D, previous);
            if error != glow::NO_ERROR {
                return Err(
                    io::Error::other(format!("OpenGL texture upload failed: {error:#x}")).into(),
                );
            }
        }
        Ok(UploadTiming { prepare, upload })
    }
}

impl Drop for TextureUpload {
    fn drop(&mut self) {
        unsafe { self.gl.delete_texture(self.texture) };
    }
}

struct App {
    runner: Runner,
    upload: TextureUpload,
    result: Arc<Mutex<Result<(), String>>>,
    finished: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.finished {
            match self.runner.step(Some(&mut self.upload)) {
                Ok(false) => {}
                outcome => {
                    *self.result.lock().expect("benchmark result") =
                        outcome.map(|_| ()).map_err(|error| error.to_string());
                    self.finished = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Playback baseline");
            ui.label("CSV results are written to stdout. Closing early fails the run.");
            if let Some(run) = &self.runner.active {
                ui.label(format!("{} / {}", run.case.name, run.case.input.name()));
                ui.label(format!("Tick {}; probe {}", run.tick, run.probe_tick));
            }
        });
        ctx.request_repaint();
    }
}

pub(super) fn run(runner: Runner) -> Result<(), Box<dyn Error>> {
    let result = Arc::new(Mutex::new(Err(
        "Benchmark window closed before completion".into()
    )));
    let app_result = Arc::clone(&result);
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        vsync: false,
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 160.0]),
        ..Default::default()
    };
    eframe::run_native(
        "IBNIZ playback baseline",
        options,
        Box::new(move |cc| {
            let gl = cc
                .gl
                .clone()
                .ok_or_else(|| io::Error::other("OpenGL context unavailable"))?;
            unsafe {
                eprintln!(
                    "OpenGL vendor={}; renderer={}; version={}",
                    gl.get_parameter_string(glow::VENDOR),
                    gl.get_parameter_string(glow::RENDERER),
                    gl.get_parameter_string(glow::VERSION)
                );
            }
            Ok(Box::new(App {
                runner,
                upload: TextureUpload::new(gl)
                    .map_err(|error| io::Error::other(error.to_string()))?,
                result: app_result,
                finished: false,
            }))
        }),
    )?;
    let outcome = result.lock().expect("benchmark result").clone();
    outcome.map_err(|error| io::Error::other(error).into())
}
