use egui::Color32;

pub fn color_from_value(value: f32, background: Color32) -> Color32 {
    colormap_with_zero_color(value, background)
}

fn colormap_with_zero_color(value: f32, zero_color: Color32) -> Color32 {
    let v = if value.is_finite() { value } else { 0.0 };
    let v = v.clamp(0.0, 1.0);
    let segment = 1.0 / 3.0;
    let blue = Color32::from_rgb(0, 0, 255);
    let green = Color32::from_rgb(0, 255, 0);
    let red = Color32::from_rgb(255, 0, 0);
    if v <= segment {
        let t = if segment == 0.0 { 0.0 } else { v / segment };
        lerp_color(zero_color, blue, t)
    } else if v <= 2.0 * segment {
        let t = (v - segment) / segment;
        lerp_color(blue, green, t)
    } else {
        let t = (v - 2.0 * segment) / segment;
        lerp_color(green, red, t)
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let clamped_t = t.clamp(0.0, 1.0);
    let ar = a.r() as f32;
    let ag = a.g() as f32;
    let ab = a.b() as f32;
    let aa = a.a() as f32;
    let br = b.r() as f32;
    let bg = b.g() as f32;
    let bb = b.b() as f32;
    let ba = b.a() as f32;
    Color32::from_rgba_unmultiplied(
        (ar + (br - ar) * clamped_t) as u8,
        (ag + (bg - ag) * clamped_t) as u8,
        (ab + (bb - ab) * clamped_t) as u8,
        (aa + (ba - aa) * clamped_t) as u8,
    )
}
