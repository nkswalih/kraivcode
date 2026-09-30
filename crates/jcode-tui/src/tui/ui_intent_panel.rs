//! Compact floating todo/worker overlay in the top-right of the chat viewport.
//!
//! Reuses the existing pinned todo band renderer so the todo card and
//! background-worker rows look exactly like the established kraivcode design.
//! No border, no gutter — just the content overlaid at half height, keeping the
//! chat full width. Hides automatically when no todos or workers exist.

use ratatui::widgets::Paragraph;

use super::*;

/// Fraction of the chat width the floating panel may occupy.
const PANEL_WIDTH_FRACTION: u16 = 2;
const PANEL_WIDTH_DENOM: u16 = 5;

/// Draw the compact todo/worker overlay into the top-right of `area` (the chat
/// viewport). Renders nothing when there are no todos and no workers.
pub(crate) fn draw_compact_panel(frame: &mut Frame, area: Rect, app: &dyn TuiState) {
    if area.width < 40 || area.height < 4 {
        return;
    }
    // Hide once everything is finished: no incomplete todos and no running or
    // failed workers. Completed-only state collapses to nothing.
    let has_active_todos = app
        .pinned_todos_payload()
        .and_then(|c| super::messages::todos_payload_parts(c))
        .map(|(todos, _, _)| todos.iter().any(|t| t.status != "completed"))
        .unwrap_or(false);
    let has_active_workers = app
        .background_task_rows()
        .iter()
        .any(|w| w.status != crate::tui::BackgroundTaskRowStatus::Completed);
    if !has_active_todos && !has_active_workers {
        return;
    }

    let panel_width = (area.width * PANEL_WIDTH_FRACTION / PANEL_WIDTH_DENOM).clamp(24, 60);
    let (mut lines, _) = super::viewport::pinned_todo_band_lines(app, panel_width, area.height);
    if lines.is_empty() {
        return;
    }

    // Half height at most: never cover the composer or the bottom status.
    let max_height = (area.height / 2).max(3) as usize;
    if lines.len() > max_height {
        lines.truncate(max_height);
        lines.push(Line::from(Span::styled(
            "  …",
            Style::default().fg(dim_color()),
        )));
    }
    let panel_height = lines.len() as u16;

    let panel_x = area
        .x
        .saturating_add(area.width.saturating_sub(panel_width));
    let panel_area = Rect {
        x: panel_x,
        y: area.y,
        width: panel_width,
        height: panel_height,
    };

    // No border, no gutter — just the content.
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, panel_area);
}
