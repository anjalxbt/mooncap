use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::app::{App, MODAL_FIELD_LABELS};

/// Main rendering function
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 3 vertical sections: header, body, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(10),   // body
            Constraint::Length(8), // log
        ])
        .split(area);

    draw_header(frame, app, main_chunks[0]);
    draw_body(frame, app, main_chunks[1]);
    draw_log(frame, app, main_chunks[2]);

    // Draw modal overlay on top if open
    if app.modal_open {
        draw_modal(frame, app, area);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(
        " 🚀 MOONCAP — {} (${}) ",
        app.token_name, app.token_symbol
    );

    let status = if app.target_hit {
        Span::styled(
            " 🔥 TARGET HIT! ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
        )
    } else {
        let progress = app.progress();
        Span::styled(
            format!(" {:.1}% to target ", progress),
            Style::default().fg(Color::Cyan),
        )
    };

    let chain_info = Span::styled(
        format!(" {} ", app.chain.to_uppercase()),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    );

    let header_line = Line::from(vec![
        chain_info,
        Span::raw(" "),
        status,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let paragraph = Paragraph::new(header_line).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    // Split body into chart (left) and stats (right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    draw_chart(frame, app, body_chunks[0]);
    draw_stats(frame, app, body_chunks[1]);
}

fn draw_chart(frame: &mut Frame, app: &App, area: Rect) {
    // Split chart area: sparkline + gauge
    let chart_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    // Sparkline
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" 📈 Market Cap History ")
        .title_style(Style::default().fg(Color::Green));

    let sparkline_color = if app.price_change_1h >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };

    let sparkline = Sparkline::default()
        .block(block)
        .data(&app.market_cap_history)
        .style(Style::default().fg(sparkline_color));

    frame.render_widget(sparkline, chart_chunks[0]);

    // Progress gauge toward target
    let progress = app.progress();
    let gauge_label = format!(
        "${:.0} / ${:.0}",
        app.market_cap, app.target_market_cap
    );

    let gauge_color = if progress >= 100.0 {
        Color::Yellow
    } else if progress >= 75.0 {
        Color::Green
    } else if progress >= 50.0 {
        Color::Cyan
    } else {
        Color::Blue
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" 🎯 Target Progress ")
                .title_style(Style::default().fg(Color::Yellow)),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
        .ratio(progress / 100.0)
        .label(gauge_label);

    frame.render_widget(gauge, chart_chunks[1]);
}

fn draw_stats(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" 📊 Stats ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let price_color = if app.price_change_1h >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };

    let change_24h_color = if app.price_change_24h >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };

    let change_1h_str = format_change(app.price_change_1h);
    let change_24h_str = format_change(app.price_change_24h);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Price       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_price(app.current_price),
                Style::default().fg(price_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Market Cap  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_dollar(app.market_cap),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  FDV         ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_dollar(app.fdv), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1h Change   ", Style::default().fg(Color::DarkGray)),
            Span::styled(change_1h_str, Style::default().fg(price_color)),
        ]),
        Line::from(vec![
            Span::styled("  24h Change  ", Style::default().fg(Color::DarkGray)),
            Span::styled(change_24h_str, Style::default().fg(change_24h_color)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Volume 24h  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_dollar(app.volume_24h), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Liquidity   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_dollar(app.liquidity_usd),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Buys  24h   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.buys_24h),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Sells 24h   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.sells_24h),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Target      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_dollar(app.target_market_cap),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 🎯"),
        ]),
        Line::from(vec![
            Span::styled("  Fetches     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.fetch_count),
                Style::default().fg(Color::White),
            ),
            if app.error_count > 0 {
                Span::styled(
                    format!("  ({} errors)", app.error_count),
                    Style::default().fg(Color::Red),
                )
            } else {
                Span::raw("")
            },
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_log(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" 📋 Log ")
        .title_style(Style::default().fg(Color::White));

    let items: Vec<ListItem> = app
        .log_messages
        .iter()
        .rev()
        .take(area.height as usize - 2)
        .map(|msg| {
            let style = if msg.contains("🔥") {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if msg.contains("❌") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(Span::styled(msg.clone(), style))
        })
        .collect();

    let help = Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow).bold()),
        Span::styled(" quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled("r", Style::default().fg(Color::Yellow).bold()),
        Span::styled(" refresh  ", Style::default().fg(Color::DarkGray)),
        Span::styled("c", Style::default().fg(Color::Yellow).bold()),
        Span::styled(" config  ", Style::default().fg(Color::DarkGray)),
        Span::styled("s", Style::default().fg(Color::Yellow).bold()),
        Span::styled(" stop alarm", Style::default().fg(Color::DarkGray)),
    ]);

    // We draw the list and the help line within the block
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let log_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let list = List::new(items);
    frame.render_widget(list, log_chunks[0]);

    let help_para = Paragraph::new(help);
    frame.render_widget(help_para, log_chunks[1]);
}

// ========== Modal Overlay ==========

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_modal(frame: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(60, 50, area);

    // Clear the area behind the modal
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" ⚙  Configure MoonCap ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Layout: fields + footer
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top padding
            Constraint::Length(2),  // field 0
            Constraint::Length(1),  // spacing
            Constraint::Length(2),  // field 1
            Constraint::Length(1),  // spacing
            Constraint::Length(2),  // field 2
            Constraint::Length(1),  // spacing
            Constraint::Length(2),  // field 3
            Constraint::Min(1),    // spacer
            Constraint::Length(1), // footer help
        ])
        .split(inner);

    let field_areas = [modal_chunks[1], modal_chunks[3], modal_chunks[5], modal_chunks[7]];

    for (i, field_area) in field_areas.iter().enumerate() {
        let is_active = i == app.modal_active_field;

        let label_style = if is_active {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let value_style = if is_active {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let cursor = if is_active { "█" } else { "" };
        let indicator = if is_active { " ▶ " } else { "   " };

        let label_line = Line::from(vec![
            Span::styled(indicator, label_style),
            Span::styled(MODAL_FIELD_LABELS[i], label_style),
        ]);

        let value_line = Line::from(vec![
            Span::raw("   "),
            Span::styled(&app.modal_fields[i], value_style),
            Span::styled(cursor, Style::default().fg(Color::Cyan)),
        ]);

        let field_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(*field_area);

        frame.render_widget(Paragraph::new(label_line), field_chunks[0]);
        frame.render_widget(Paragraph::new(value_line), field_chunks[1]);
    }

    // Footer
    let footer = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Green).bold()),
        Span::styled(" confirm  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab/↓", Style::default().fg(Color::Yellow).bold()),
        Span::styled(" next  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Shift+Tab/↑", Style::default().fg(Color::Yellow).bold()),
        Span::styled(" prev  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Red).bold()),
        Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(footer), modal_chunks[9]);
}

// ========== Formatting Helpers ==========

fn format_dollar(val: f64) -> String {
    if val >= 1_000_000.0 {
        format!("${:.2}M", val / 1_000_000.0)
    } else if val >= 1_000.0 {
        format!("${:.1}K", val / 1_000.0)
    } else {
        format!("${:.2}", val)
    }
}

fn format_price(val: f64) -> String {
    if val >= 1.0 {
        format!("${:.4}", val)
    } else if val >= 0.01 {
        format!("${:.6}", val)
    } else {
        format!("${:.10}", val)
    }
}

fn format_change(val: f64) -> String {
    if val >= 0.0 {
        format!("+{:.2}%", val)
    } else {
        format!("{:.2}%", val)
    }
}
