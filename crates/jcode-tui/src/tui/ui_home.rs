use super::{TuiState, dim_color};
use ratatui::{prelude::*, widgets::Paragraph};
use unicode_width::UnicodeWidthStr;

fn logo_yellow() -> Color {
    crate::tui::color_support::rgb(255, 210, 0)
}

/// Full double-line box-drawing wordmark, 6 rows × 69 columns.
/// Rows 2-3 are padded with trailing spaces to match the 69-col width.
const WORDMARK: [&str; 6] = [
    "██╗  ██╗██████╗  █████╗ ██╗██╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗",
    "██║ ██╔╝██╔══██╗██╔══██╗██║██║   ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝",
    "█████╔╝ ██████╔╝███████║██║██║   ██║██║     ██║   ██║██║  ██║█████╗  ",
    "██╔═██╗ ██╔══██╗██╔══██║██║╚██╗ ██╔╝██║     ██║   ██║██║  ██║██╔══╝  ",
    "██║  ██╗██║  ██║██║  ██║██║ ╚████╔╝ ╚██████╗╚██████╔╝██████╔╝███████╗",
    "╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝",
];

/// Subtitle below the wordmark.
const SUBTITLE: &str = "CODE \u{25AA} THINK \u{25AA} PLAN \u{25AA} BUILD";

/// Build the logo lines. Falls back to plain "KRAIVCODE" when too narrow/short.
fn build_logo_lines(width: u16, height: u16) -> Vec<Line<'static>> {
    let wordmark_w = WORDMARK
        .iter()
        .map(|r| UnicodeWidthStr::width(*r))
        .max()
        .unwrap_or(0);
    if (width as usize) < (wordmark_w + 4) || height < (WORDMARK.len() as u16 + 3) {
        return vec![Line::from(Span::styled(
            "KRAIVCODE",
            Style::default().fg(logo_yellow()).bold(),
        ))];
    }

    let yellow = Style::default().fg(logo_yellow()).bold();
    WORDMARK
        .iter()
        .map(|row| Line::from(Span::styled(*row, yellow)))
        .collect()
}

pub(super) fn draw_home(
    frame: &mut Frame,
    _app: &dyn TuiState,
    area: Rect,
    input_height: u16,
) -> Rect {
    let input_height = input_height.max(1);
    let composer_height = input_height;
    let composer_width = area.width.clamp(40, 88);

    let logo_lines = build_logo_lines(area.width, area.height);
    let logo_h = logo_lines.len() as u16;

    let used_height = 1 + logo_h + 1 + 1 + composer_height + 1;
    let top_pad = area.height.saturating_sub(used_height) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(logo_h),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(composer_height),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(logo_lines).alignment(Alignment::Center),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            SUBTITLE,
            Style::default().fg(logo_yellow()).dim(),
        )))
        .alignment(Alignment::Center),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "What would you like to build?",
            Style::default().fg(dim_color()),
        )))
        .alignment(Alignment::Center),
        chunks[3],
    );

    let composer_x = area.x + area.width.saturating_sub(composer_width) / 2;

    let outer = Rect::new(
        composer_x,
        chunks[4].y,
        composer_width.min(area.width),
        chunks[4].height,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[", Style::default().fg(dim_color())),
            Span::raw(" "),
            Span::styled("/models", Style::default().fg(logo_yellow()).bold()),
            Span::raw(" "),
            Span::styled("]", Style::default().fg(dim_color())),
            Span::raw("   "),
            Span::styled("|", Style::default().fg(dim_color())),
            Span::raw("   "),
            Span::styled("[", Style::default().fg(dim_color())),
            Span::raw(" "),
            Span::styled("/agents", Style::default().fg(logo_yellow()).bold()),
            Span::raw(" "),
            Span::styled("]", Style::default().fg(dim_color())),
            Span::raw("   "),
            Span::styled("|", Style::default().fg(dim_color())),
            Span::raw("   "),
            Span::styled("[", Style::default().fg(dim_color())),
            Span::raw(" "),
            Span::styled("/sessions", Style::default().fg(logo_yellow()).bold()),
            Span::raw(" "),
            Span::styled("]", Style::default().fg(dim_color())),
            Span::raw("   "),
            Span::styled("|", Style::default().fg(dim_color())),
            Span::raw("   "),
            Span::styled("[", Style::default().fg(dim_color())),
            Span::raw(" "),
            Span::styled("/help", Style::default().fg(logo_yellow()).bold()),
            Span::raw(" "),
            Span::styled("]", Style::default().fg(dim_color())),
        ]))
        .alignment(Alignment::Center),
        chunks[5],
    );

    outer
}
