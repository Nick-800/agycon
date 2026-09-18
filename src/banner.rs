fn interpolate_color(stops: &[(f32, (u8, u8, u8))], t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    for i in 0..stops.len().saturating_sub(1) {
        let (t0, c0) = stops[i];
        let (t1, c1) = stops[i + 1];
        if t >= t0 && t <= t1 {
            let ratio = if (t1 - t0).abs() < f32::EPSILON {
                0.0
            } else {
                (t - t0) / (t1 - t0)
            };
            let r = (c0.0 as f32 + ratio * (c1.0 as f32 - c0.0 as f32)).round() as u8;
            let g = (c0.1 as f32 + ratio * (c1.1 as f32 - c0.1 as f32)).round() as u8;
            let b = (c0.2 as f32 + ratio * (c1.2 as f32 - c0.2 as f32)).round() as u8;
            return (r, g, b);
        }
    }
    stops.last().map(|s| s.1).unwrap_or((255, 255, 255))
}

pub fn print_banner() {
    let stops: &[(f32, (u8, u8, u8))] = &[
        (0.00, (66, 133, 244)), // Azure Blue
        (0.25, (52, 168, 83)),  // Emerald Green
        (0.50, (255, 140, 20)), // Amber Orange
        (0.75, (234, 67, 53)),  // Coral Red
        (1.00, (66, 133, 244)), // Return to Azure Blue
    ];

    let lines = [
        r"     ╭───╮        ______ __  __ ______ ____   _   __",
        r"    ╱     ╲      / ____/ \ \/ // ____// __ \ / | / /",
        r"   ╱   ╭╮  ╲    / / __    \  // /    / / / //  |/ / ",
        r"  ╱   ╱  ╲  ╲  / /_/ /    / // /___ / /_/ // /|  /  ",
        r" ╰───╯    ╰──╯  \____/   /_/ \____/ \____//_/ |_/   ",
    ];

    let max_len = lines.iter().map(|l| l.len()).max().unwrap_or(1) as f32;

    println!();
    for line in &lines {
        let mut output = String::new();
        for (col_idx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                output.push(' ');
            } else {
                let t = (col_idx as f32) / max_len;
                let (r, g, b) = interpolate_color(stops, t);
                output.push_str(&format!("\x1b[38;2;{r};{g};{b};1m{ch}\x1b[0m"));
            }
        }
        println!("{output}");
    }

    println!("\x1b[2m   Antigravity Conversation Launcher\x1b[0m\n");
}
