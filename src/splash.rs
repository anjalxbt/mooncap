use std::time::Duration;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const ROCKET_FRAMES: [&[&str]; 4] = [
    &[
        "        ",
        "   /\\   ",
        "  /  \\  ",
        " | 🌙 | ",
        " |    | ",
        " |    | ",
        " /|  |\\ ",
        "/_|__|_\\",
        "  |\\/|  ",
        "  |\\/|  ",
        "  ''''  ",
    ],
    &[
        "        ",
        "   /\\   ",
        "  /  \\  ",
        " | 🌙 | ",
        " |    | ",
        " |    | ",
        " /|  |\\ ",
        "/_|__|_\\",
        "  |**|  ",
        " .*||*. ",
        " *''''* ",
        "  *  *  ",
    ],
    &[
        "        ",
        "   /\\   ",
        "  /  \\  ",
        " | 🌙 | ",
        " |    | ",
        " |    | ",
        " /|  |\\ ",
        "/_|__|_\\",
        "  |##|  ",
        " *|##|* ",
        ".*''''*.",
        "* *  * *",
        " *    * ",
        "  *  *  ",
    ],
    &[
        "        ",
        "   /\\   ",
        "  /  \\  ",
        " | 🌙 | ",
        " |    | ",
        " |    | ",
        " /|  |\\ ",
        "/_|__|_\\",
        "  |##|  ",
        " *|##|* ",
        "**''''**",
        "* *##* *",
        " **##** ",
        "  *##*  ",
        "   **   ",
        "    *   ",
    ],
];

const LOGO_TEXT: &[&str] = &[
    " ███╗   ███╗ ██████╗  ██████╗ ███╗   ██╗ ██████╗ █████╗ ██████╗ ",
    " ████╗ ████║██╔═══██╗██╔═══██╗████╗  ██║██╔════╝██╔══██╗██╔══██╗",
    " ██╔████╔██║██║   ██║██║   ██║██╔██╗ ██║██║     ███████║██████╔╝",
    " ██║╚██╔╝██║██║   ██║██║   ██║██║╚██╗██║██║     ██╔══██║██╔═══╝ ",
    " ██║ ╚═╝ ██║╚██████╔╝╚██████╔╝██║ ╚████║╚██████╗██║  ██║██║     ",
    " ╚═╝     ╚═╝ ╚═════╝  ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝╚═╝     ",
];

const STARS: &[(u16, u16)] = &[
    (5, 2), (15, 4), (30, 1), (45, 3), (60, 5), (70, 2), (10, 8),
    (25, 6), (50, 7), (65, 4), (35, 9), (55, 1), (40, 6), (20, 3),
    (75, 8), (8, 5), (48, 9), (62, 3), (22, 7), (38, 2), (58, 6),
    (12, 1), (42, 4), (68, 7), (28, 8), (52, 2), (18, 6), (72, 5),
];

struct AnimState {
    frame: usize,
    rocket_y: i16,
    logo_alpha: u8,
    stars_twinkle: usize,
}

/// Run the splash animation. Returns after ~2 seconds.
pub fn run_splash(terminal: &mut ratatui::DefaultTerminal) {
    let total_steps = 30; // ~2 seconds at 65ms per frame
    let frame_delay = Duration::from_millis(65);

    for step in 0..total_steps {
        let area = terminal.size().unwrap_or_default();

        let state = AnimState {
            frame: step % ROCKET_FRAMES.len(),
            rocket_y: compute_rocket_y(step, total_steps, area.height),
            logo_alpha: compute_logo_alpha(step, total_steps),
            stars_twinkle: step,
        };

        let _ = terminal.draw(|f| draw_splash(f, &state));
        std::thread::sleep(frame_delay);
    }

    // Hold final frame briefly
    std::thread::sleep(Duration::from_millis(300));
}

fn compute_rocket_y(step: usize, total: usize, height: u16) -> i16 {
    // Rocket starts at bottom and flies up past the top
    let start = height as i16;
    let end = -18_i16;
    let progress = step as f64 / total as f64;
    // Ease-in curve (accelerating)
    let eased = progress * progress;
    start + ((end - start) as f64 * eased) as i16
}

fn compute_logo_alpha(step: usize, total: usize) -> u8 {
    // Logo fades in during the last 40% of the animation
    let fade_start = total * 60 / 100;
    if step < fade_start {
        0
    } else {
        let progress = (step - fade_start) as f64 / (total - fade_start) as f64;
        (progress * 255.0).min(255.0) as u8
    }
}

fn draw_splash(frame: &mut Frame, state: &AnimState) {
    let area = frame.area();

    // Black background
    let bg = Paragraph::new("");
    frame.render_widget(bg, area);

    // Draw twinkling stars
    draw_stars(frame, area, state.stars_twinkle);

    // Draw rocket at current position
    let rocket_art = ROCKET_FRAMES[state.frame];
    draw_rocket(frame, area, rocket_art, state.rocket_y);

    // Draw logo (fades in)
    if state.logo_alpha > 0 {
        draw_logo(frame, area, state.logo_alpha);
    }
}

fn draw_stars(frame: &mut Frame, area: Rect, tick: usize) {
    for (i, &(x, y)) in STARS.iter().enumerate() {
        if x < area.width && y < area.height {
            let twinkle = (tick + i) % 4;
            let (ch, color) = match twinkle {
                0 => ("·", Color::DarkGray),
                1 => ("✦", Color::White),
                2 => ("·", Color::Gray),
                _ => ("✧", Color::Cyan),
            };

            let star = Paragraph::new(Span::styled(
                ch,
                Style::default().fg(color),
            ));
            let star_area = Rect::new(x, y, 1, 1);
            frame.render_widget(star, star_area);
        }
    }
}

fn draw_rocket(frame: &mut Frame, area: Rect, art: &[&str], y_offset: i16) {
    let rocket_width = 16_u16;
    let x = area.width.saturating_sub(rocket_width) / 2;

    for (i, line) in art.iter().enumerate() {
        let y = y_offset + i as i16;
        if y >= 0 && (y as u16) < area.height {
            // Color the exhaust lines differently
            let is_exhaust = i >= 8;
            let style = if is_exhaust {
                let exhaust_colors = [Color::Yellow, Color::Red, Color::Rgb(255, 100, 0), Color::Rgb(255, 50, 0)];
                let color_idx = (i - 8) % exhaust_colors.len();
                Style::default().fg(exhaust_colors[color_idx])
            } else if i <= 2 {
                // Nose cone
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                // Body
                Style::default().fg(Color::Cyan)
            };

            let span = Span::styled(*line, style);
            let line_widget = Paragraph::new(span);
            let line_area = Rect::new(x, y as u16, rocket_width, 1);
            frame.render_widget(line_widget, line_area);
        }
    }
}

fn draw_logo(frame: &mut Frame, area: Rect, alpha: u8) {
    let logo_height = LOGO_TEXT.len() as u16;
    let logo_width = LOGO_TEXT[0].chars().count() as u16;

    // Center the logo
    let x = area.width.saturating_sub(logo_width) / 2;
    let y = area.height.saturating_sub(logo_height) / 2;

    // Map alpha to color intensity
    let intensity = alpha;
    let color = Color::Rgb(0, intensity, intensity); // cyan fade-in

    let lines: Vec<Line> = LOGO_TEXT
        .iter()
        .map(|l| {
            Line::from(Span::styled(
                *l,
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();

    let logo = Paragraph::new(lines);

    // Only render if it fits
    if x + logo_width <= area.width && y + logo_height <= area.height {
        let logo_area = Rect::new(x, y, logo_width, logo_height);
        frame.render_widget(logo, logo_area);
    }

    // Tagline below logo
    if alpha > 128 {
        let tagline = "monitoring your moonshots";
        let tag_x = area.width.saturating_sub(tagline.len() as u16) / 2;
        let tag_y = y + logo_height + 1;
        if tag_y < area.height {
            let tag = Paragraph::new(Span::styled(
                tagline,
                Style::default().fg(Color::DarkGray),
            ));
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(tag_x),
                    Constraint::Length(tagline.len() as u16),
                    Constraint::Min(0),
                ])
                .split(Rect::new(0, tag_y, area.width, 1));
            frame.render_widget(tag, chunks[1]);
        }
    }
}
