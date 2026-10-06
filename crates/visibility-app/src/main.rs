//! Desktop viewer for the `VisibilityEngine2D` algorithms.

// Release builds on Windows are GUI apps: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod camera;
mod compute;
mod gpu;
mod theme;

use std::path::PathBuf;
use std::process::ExitCode;

use eframe::egui;
use visibility_core::{SceneConfig, Vec2};

use crate::compute::{Mode, ViewSettings};

const USAGE: &str = "\
VisibilityEngine2D: interactive 2D visibility, frustum culling and occlusion culling

Usage: visibility-engine-2d [options]

View:
  --mode <shadow|frustum|occlusion>   What to compute (default: shadow)
  --viewer <X,Y>                      Viewer position in world units
  --range <R>                         View range (default: 300)
  --fov <DEG>                         Cone angle for frustum and occlusion modes (default: 90)
  --direction <DEG>                   Cone direction, 0 = right, 90 = down (default: 0)
  --rays                              Show the rays toward obstacle corners

Scene:
  --obstacles <N>                     Number of random obstacles (default: 500)
  --world <SIZE>                      World width and height (default: 2000)
  --seed <N>                          Random seed (default: 24301)

Window:
  --window <WxH>                      Window size in points (default: 1440x900)
  --zoom <Z>                          Initial zoom, screen points per world unit (default: 1)
  --center <X,Y>                      World point at the canvas center (default: the viewer)
  --theme <light|dark>                Force a color theme
  --hide-panel                        Hide the side panel
  --screenshot <FILE.png>             Render, save a screenshot, and exit

  -h, --help                          Print this help
  -V, --version                       Print the version";

/// Settings from the command line.
#[derive(Clone, Debug)]
pub struct LaunchOptions {
    /// Mode and view cone.
    pub view: ViewSettings,
    /// Random scene to generate.
    pub scene: SceneConfig,
    /// Viewer position (default: a free spot near the world center).
    pub viewer: Option<Vec2>,
    /// Initial zoom in screen points per world unit.
    pub zoom: Option<f64>,
    /// World point at the canvas center.
    pub center: Option<Vec2>,
    /// Window size in points.
    pub window: [f32; 2],
    /// Forced color theme (default: follow the system).
    pub theme: Option<egui::Theme>,
    /// Start with the side panel hidden.
    pub hide_panel: bool,
    /// Save a screenshot here and exit.
    pub screenshot: Option<PathBuf>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            view: ViewSettings::default(),
            scene: SceneConfig::default(),
            viewer: None,
            zoom: None,
            center: None,
            window: [1440.0, 900.0],
            theme: None,
            hide_panel: false,
            screenshot: None,
        }
    }
}

enum Parsed {
    Run(LaunchOptions),
    Exit(ExitCode),
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Parsed, String> {
    let mut o = LaunchOptions::default();
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(Parsed::Exit(ExitCode::SUCCESS));
            }
            "-V" | "--version" => {
                println!("visibility-engine-2d {}", env!("CARGO_PKG_VERSION"));
                return Ok(Parsed::Exit(ExitCode::SUCCESS));
            }
            "--mode" => o.view.mode = Mode::parse(&value()?).ok_or("--mode must be shadow, frustum or occlusion")?,
            "--viewer" => o.viewer = Some(parse_point(&value()?)?),
            "--range" => o.view.range = parse_num(&value()?, 1.0, 1e6)?,
            "--fov" => o.view.fov_deg = parse_num(&value()?, 1.0, 360.0)?,
            "--direction" => o.view.direction_deg = parse_num(&value()?, -360.0, 360.0)?,
            "--rays" => o.view.show_rays = true,
            "--obstacles" => o.scene.polygon_count = parse_num(&value()?, 0.0, 1e6)? as usize,
            "--world" => {
                let size = parse_num(&value()?, 100.0, 1e5)?;
                (o.scene.width, o.scene.height) = (size, size);
            }
            "--seed" => o.scene.seed = value()?.parse().map_err(|_| "--seed must be a whole number")?,
            "--window" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--window must look like 1440x900")?;
                o.window = [
                    parse_num(w, 320.0, 10_000.0)? as f32,
                    parse_num(h, 240.0, 10_000.0)? as f32,
                ];
            }
            "--zoom" => o.zoom = Some(parse_num(&value()?, camera::MIN_ZOOM, camera::MAX_ZOOM)?),
            "--center" => o.center = Some(parse_point(&value()?)?),
            "--theme" => {
                o.theme = Some(match value()?.as_str() {
                    "light" => egui::Theme::Light,
                    "dark" => egui::Theme::Dark,
                    _ => return Err("--theme must be light or dark".into()),
                });
            }
            "--hide-panel" => o.hide_panel = true,
            "--screenshot" => o.screenshot = Some(PathBuf::from(value()?)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(Parsed::Run(o))
}

fn parse_num(s: &str, min: f64, max: f64) -> Result<f64, String> {
    let v: f64 = s.trim().parse().map_err(|_| format!("'{s}' is not a number"))?;
    if v.is_finite() && (min..=max).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{v} is outside {min}..={max}"))
    }
}

fn parse_point(s: &str) -> Result<Vec2, String> {
    let (x, y) = s.split_once(',').ok_or_else(|| format!("'{s}' must look like X,Y"))?;
    Ok(Vec2::new(parse_num(x, -1e6, 1e6)?, parse_num(y, -1e6, 1e6)?))
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(Parsed::Run(o)) => o,
        Ok(Parsed::Exit(code)) => return code,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("VisibilityEngine2D")
        .with_app_id("visibility-engine-2d")
        .with_inner_size(options.window)
        .with_min_inner_size([720.0, 480.0]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let native = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        multisampling: gpu::MSAA_SAMPLES as u16,
        ..Default::default()
    };

    let result = eframe::run_native(
        "VisibilityEngine2D",
        native,
        Box::new(move |cc| Ok(Box::new(app::VisibilityApp::new(cc, options)?))),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<LaunchOptions, String> {
        match parse_args(args.iter().map(|s| (*s).to_owned()))? {
            Parsed::Run(o) => Ok(o),
            Parsed::Exit(_) => Err("exited".into()),
        }
    }

    #[test]
    fn parses_view_and_scene_options() {
        let o = parse(&[
            "--mode",
            "occlusion",
            "--viewer",
            "10,20.5",
            "--fov",
            "120",
            "--world",
            "5000",
            "--rays",
        ])
        .unwrap();
        assert_eq!(o.view.mode, Mode::Occlusion);
        assert_eq!(o.viewer, Some(Vec2::new(10.0, 20.5)));
        assert_eq!(o.view.fov_deg, 120.0);
        assert_eq!((o.scene.width, o.scene.height), (5000.0, 5000.0));
        assert!(o.view.show_rays);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(parse(&["--fov", "0"]).is_err());
        assert!(parse(&["--mode", "x"]).is_err());
        assert!(parse(&["--window", "800"]).is_err());
        assert!(parse(&["--range"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
    }
}
