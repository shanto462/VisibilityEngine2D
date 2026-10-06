//! Colors for the canvas, in light and dark mode.

use eframe::egui::Color32;

/// Obstacle colors (CSS named colors, from cornflower blue to medium turquoise).
const PALETTE: [[u8; 3]; 10] = [
    [100, 149, 237], // CornflowerBlue
    [240, 128, 128], // LightCoral
    [102, 205, 170], // MediumAquamarine
    [250, 250, 210], // LightGoldenrodYellow
    [219, 112, 147], // PaleVioletRed
    [147, 112, 219], // MediumPurple
    [32, 178, 170],  // LightSeaGreen
    [255, 218, 185], // PeachPuff
    [205, 92, 92],   // IndianRed
    [72, 209, 204],  // MediumTurquoise
];

/// Straight sRGB color of palette entry `index` (wraps around).
pub fn palette_rgba(index: u8) -> [u8; 4] {
    let [r, g, b] = PALETTE[index as usize % PALETTE.len()];
    [r, g, b, 255]
}

/// Canvas colors for one theme.
#[derive(Clone, Copy, Debug)]
pub struct CanvasColors {
    /// Area outside the world.
    pub backdrop: Color32,
    /// The world itself.
    pub paper: Color32,
    pub grid_minor: Color32,
    pub grid_major: Color32,
    /// Obstacle outlines (straight sRGB, used by the GPU renderer).
    pub outline: [f32; 4],
    pub area_fill: Color32,
    pub area_stroke: Color32,
    pub seen_fill: Color32,
    pub seen_stroke: Color32,
    pub ray_visible: Color32,
    pub ray_blocked: Color32,
    pub viewer_fill: Color32,
    pub viewer_stroke: Color32,
    pub hover_stroke: Color32,
    /// Viewer marker and label when the viewer stands inside an obstacle.
    pub warning: Color32,
}

impl CanvasColors {
    pub fn new(dark: bool) -> Self {
        let shared = Self {
            backdrop: Color32::from_rgb(233, 236, 239),
            paper: Color32::WHITE,
            grid_minor: Color32::from_rgb(226, 229, 233),
            grid_major: Color32::from_rgb(196, 201, 208),
            outline: [0.0, 0.0, 0.0, 0.55],
            area_fill: Color32::from_rgba_unmultiplied(255, 214, 0, 80),
            area_stroke: Color32::from_rgb(245, 190, 0),
            seen_fill: Color32::from_rgba_unmultiplied(17, 24, 39, 200),
            seen_stroke: Color32::from_rgb(255, 170, 0),
            ray_visible: Color32::from_rgba_unmultiplied(22, 163, 74, 150),
            ray_blocked: Color32::from_rgba_unmultiplied(220, 38, 38, 150),
            viewer_fill: Color32::from_rgba_unmultiplied(0, 150, 255, 60),
            viewer_stroke: Color32::from_rgb(0, 100, 200),
            hover_stroke: Color32::from_rgb(17, 24, 39),
            warning: Color32::from_rgb(220, 38, 38),
        };
        if dark {
            Self {
                backdrop: Color32::from_rgb(14, 16, 21),
                paper: Color32::from_rgb(27, 31, 39),
                grid_minor: Color32::from_rgb(36, 41, 51),
                grid_major: Color32::from_rgb(52, 59, 72),
                area_fill: Color32::from_rgba_unmultiplied(255, 214, 0, 60),
                seen_fill: Color32::from_rgba_unmultiplied(8, 10, 14, 215),
                viewer_stroke: Color32::from_rgb(80, 170, 255),
                hover_stroke: Color32::WHITE,
                ..shared
            }
        } else {
            shared
        }
    }
}
