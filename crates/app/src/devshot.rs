//! Dev aid: `W3HK_SCREENSHOT=out.png` saves a screenshot once the UI has
//! settled (game art loaded), then closes the app.

use std::path::PathBuf;

use eframe::egui::{self, ViewportCommand};

pub struct DevShot {
    path: PathBuf,
    settled_frames: u32,
    requested: bool,
}

impl DevShot {
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("W3HK_SCREENSHOT")?;
        Some(DevShot { path: path.into(), settled_frames: 0, requested: false })
    }

    /// Call every frame. `settled` = nothing is still loading.
    pub fn tick(&mut self, ctx: &egui::Context, settled: bool) {
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            let [w, h] = image.size;
            if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, bytes)
                && let Err(e) = img.save(&self.path) {
                    eprintln!("screenshot save failed: {e}");
                }
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        if !settled {
            ctx.request_repaint();
            return;
        }
        self.settled_frames += 1;
        if self.settled_frames >= 5 && !self.requested {
            self.requested = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        ctx.request_repaint();
    }
}
