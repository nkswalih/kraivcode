use super::{dim_color, header_name_color, TuiState};
use ratatui::{
    prelude::*,
    widgets::Paragraph,
};

pub(super) fn draw_home(
    frame: &mut Frame,
    _app: &dyn TuiState,
    area: Rect,
    input_height: u16,
) -> Rect {
    let input_height = input_height.max(1);
    // input_height already includes the 2 border rows (top + bottom)
    // computed by draw_input's .block() geometry; do not double-count.
    let composer_height = input_height;

    let composer_width = area.width.clamp(40, 88);

    let used_height = 1 + 1 + composer_height + 1;
    let top_pad = area.height.saturating_sub(used_height) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(composer_height),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "KRAIVCODE",
            Style::default()
                .fg(header_name_color())
                .bold(),
        )))
        .alignment(Alignment::Center),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "What would you like to build?",
            Style::default().fg(dim_color()),
        )))
        .alignment(Alignment::Center),
        chunks[2],
    );

    let composer_x = area.x + area.width.saturating_sub(composer_width) / 2;

    // The draw_input function renders the single canonical rounded border.
    // Return the outer rect so draw_input owns the border.
    let outer = Rect::new(
        composer_x,
        chunks[3].y,
        composer_width.min(area.width),
        chunks[3].height,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("/models", Style::default().fg(header_name_color())),
            Span::raw("    "),
            Span::styled("/agents", Style::default().fg(header_name_color())),
            Span::raw("    "),
            Span::styled("/sessions", Style::default().fg(header_name_color())),
            Span::raw("    "),
            Span::styled("/help", Style::default().fg(header_name_color())),
        ]))
        .alignment(Alignment::Center),
        chunks[4],
    );

    outer
}