//! Sticky right-side Intent/Todo/Workers panel.
//!
//! A single live component that separates the agent's plan into three visually
//! distinct sections, reusing the existing todo-payload and background-worker
//! state (no new worker system, no transcript duplication).

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use super::*;
use crate::todo::TodoItem;

/// Maximum worker rows shown before collapsing into a "+N more" summary line.
const MAX_WORKER_ROWS: usize = 4;
/// Maximum todo rows shown before collapsing into a "+N more" summary line.
const MAX_TODO_ROWS: usize = 8;
/// Minimum panel width below which we degrade to plain labels.
const MIN_USABLE_WIDTH: u16 = 24;

/// Draw the Intent/Todo/Workers panel into the given area, using a left-aligned
/// side-panel border style that matches the rest of the kraivcode theme.
pub(crate) fn draw_intent_panel(
    frame: &mut Frame,
    area: Rect,
    app: &dyn TuiState,
) {
    let border_style = Style::default().fg(dim_color());
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::LEFT)
        .border_style(border_style);
    let inner = block.inner(area);
    let lines = build_intent_todo_workers_lines(app, area.width, area.height);
    if let Some(lines) = lines {
        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, inner);
    } else {
        // No content: still render the border to keep the layout stable, or
        // skip entirely. Rendering an empty paragraph is harmless.
        frame.render_widget(Paragraph::new(Vec::<Line>::new()).block(block), inner);
    }
}

/// Render the Intent/Todo/Workers panel lines for the right side panel.
///
/// Returns `None` when there is nothing to show (no todos and no workers), so
/// callers can skip opening the side pane entirely.
fn build_intent_todo_workers_lines(
    app: &dyn TuiState,
    width: u16,
    _height: u16,
) -> Option<Vec<Line<'static>>> {
    if width < MIN_USABLE_WIDTH {
        return None;
    }
    let content = app.pinned_todos_payload()?;
    let (todos, plan, _goals) = super::messages::todos_payload_parts(content)?;
    let workers = app.background_task_rows();
    render_panel_lines(&todos, &plan, workers, width, app.animation_elapsed())
}

/// Pure panel builder: given the plan, todo items and background workers, build
/// the INTENT / TODO / WORKERS lines. Kept separate so tests can drive it with
/// fixture data without a full `TuiState`.
fn render_panel_lines(
    todos: &[TodoItem],
    plan: &crate::todo::TodoPlan,
    workers: &[crate::tui::BackgroundTaskRow],
    width: u16,
    animation_elapsed: f32,
) -> Option<Vec<Line<'static>>> {
    if width < MIN_USABLE_WIDTH {
        return None;
    }
    let todo_done = todos.iter().filter(|t| t.status == "completed").count();
    let todo_total = todos.len();
    let worker_done = workers
        .iter()
        .filter(|w| w.status == crate::tui::BackgroundTaskRowStatus::Completed)
        .count();
    let worker_total = workers.len();

    if todo_total == 0 && worker_total == 0 {
        return None;
    }

    let inner_width = (width as usize).saturating_sub(2).max(1);
    let mut lines: Vec<Line<'static>> = Vec::new();

    // ---- INTENT section ----
    push_section_header(&mut lines, "INTENT", inner_width);
    let intent = plan
        .user_intention
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("No intent recorded yet");
    push_wrapped_text(&mut lines, intent, inner_width, asap_color());

    lines.push(Line::from(""));

    // ---- TODO section ----
    push_section_header(&mut lines, "TODO", inner_width);
    let todo_over = todo_total.saturating_sub(MAX_TODO_ROWS);
    let todo_shown = todo_total.min(MAX_TODO_ROWS);
    for todo in todos.iter().take(todo_shown) {
        lines.push(todo_item_line(todo, inner_width, animation_elapsed));
    }
    if todo_over > 0 {
        lines.push(Line::from(Span::styled(
            format!("  +{todo_over} more (todo)"),
            Style::default().fg(dim_color()),
        )));
    }
    lines.push(Line::from(""));

    // ---- WORKERS section ----
    push_section_header(&mut lines, "WORKERS", inner_width);
    let worker_over = worker_total.saturating_sub(MAX_WORKER_ROWS);
    let worker_shown = worker_total.min(MAX_WORKER_ROWS);
    for worker in workers.iter().take(worker_shown) {
        lines.push(worker_item_line(worker, inner_width, animation_elapsed));
    }
    if worker_over > 0 {
        lines.push(Line::from(Span::styled(
            format!("  +{worker_over} more (workers)"),
            Style::default().fg(dim_color()),
        )));
    }
    lines.push(Line::from(""));

    // ---- Footer ----
    lines.push(Line::from(Span::styled(
        format!("{todo_done}/{todo_total} todo · {worker_done}/{worker_total} workers"),
        Style::default().fg(dim_color()),
    )));

    Some(lines)
}

fn push_section_header(lines: &mut Vec<Line<'static>>, label: &str, _inner_width: usize) {
    lines.push(Line::from(vec![
        Span::styled("▌ ".to_string(), Style::default().fg(accent_color())),
        Span::styled(label.to_string(), Style::default().fg(accent_color()).bold()),
    ]));
}

fn push_wrapped_text(
    lines: &mut Vec<Line<'static>>,
    text: &str,
    inner_width: usize,
    color: Color,
) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let mut current = String::new();
    for word in text.split_whitespace() {
        let sep = if current.is_empty() { "" } else { " " };
        let candidate = format!("{current}{sep}{word}");
        if UnicodeWidthStr::width(candidate.as_str()) > inner_width && !current.is_empty() {
            lines.push(Line::from(Span::styled(current, Style::default().fg(color))));
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(Line::from(Span::styled(current, Style::default().fg(color))));
    }
}

fn todo_item_line(todo: &TodoItem, inner_width: usize, animation_elapsed: f32) -> Line<'static> {
    let (glyph, glyph_color, text_color) =
        if !todo.blocked_by.is_empty() && todo.status != "completed" {
            ("⊳", super::messages::todo_warning_color(), super::messages::todo_label_color())
        } else {
            match todo.status.as_str() {
                "completed" => ("✓", rgb(105, 190, 125), rgb(135, 150, 145)),
                "in_progress" => (
                    running_spinner(animation_elapsed),
                    asap_color(),
                    rgb(225, 232, 240),
                ),
                "cancelled" => ("✕", rgb(190, 105, 115), rgb(145, 130, 135)),
                _ => ("○", asap_color(), rgb(195, 202, 212)),
            }
        };
    let content = truncate_to_width(&todo.content, inner_width.saturating_sub(4).max(1));
    Line::from(vec![
        Span::styled(format!("  {glyph} "), Style::default().fg(glyph_color)),
        Span::styled(content, Style::default().fg(text_color)),
    ])
}

fn worker_item_line(
    worker: &crate::tui::BackgroundTaskRow,
    inner_width: usize,
    animation_elapsed: f32,
) -> Line<'static> {
    let (glyph, glyph_color, suffix) = match worker.status {
        crate::tui::BackgroundTaskRowStatus::Running => (
            running_spinner(animation_elapsed),
            accent_color(),
            format!(" {}", percent_label(worker.percent)),
        ),
        crate::tui::BackgroundTaskRowStatus::Completed => ("✓", rgb(105, 190, 125), String::new()),
        crate::tui::BackgroundTaskRowStatus::Failed => {
            ("✕", rgb(225, 105, 105), "  failed".to_string())
        }
    };
    let label = truncate_to_width(&worker.label, inner_width.saturating_sub(6).max(1));
    Line::from(vec![
        Span::styled(format!("  {glyph} bg "), Style::default().fg(glyph_color)),
        Span::styled(label, Style::default().fg(super::messages::todo_label_color())),
        Span::styled(suffix, Style::default().fg(dim_color())),
    ])
}

/// 360-degree rotating quarter-circle spinner for in-progress items.
fn running_spinner(elapsed: f32) -> &'static str {
    const FRAMES: [&'static str; 4] = ["◐", "◓", "◑", "◒"];
    FRAMES[((elapsed * 4.0) as usize) % FRAMES.len()]
}

fn percent_label(percent: Option<f32>) -> String {
    match percent {
        Some(p) => format!("{}%", p.round() as u8),
        None => "running".to_string(),
    }
}

fn truncate_to_width(text: &str, width: usize) -> String {
    let text = text.replace(['\r', '\n'], " ");
    if UnicodeWidthStr::width(text.as_str()) <= width {
        return text;
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_string();
    }
    let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::BackgroundTaskRowStatus;

    fn todo(content: &str, status: &str) -> TodoItem {
        TodoItem {
            content: content.to_string(),
            status: status.to_string(),
            priority: String::new(),
            id: String::new(),
            group: None,
            confidence: None,
            completion_confidence: None,
            confidence_history: Vec::new(),
            blocked_by: Vec::new(),
            assigned_to: None,
        }
    }

    fn worker(label: &str, status: BackgroundTaskRowStatus) -> crate::tui::BackgroundTaskRow {
        crate::tui::BackgroundTaskRow {
            task_id: label.to_string(),
            label: label.to_string(),
            percent: None,
            status,
            completed_at: None,
        }
    }

    fn line_text(line: &Line<'static>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn empty_state_returns_none() {
        let plan = crate::todo::TodoPlan::default();
        assert!(render_panel_lines(&[], &plan, &[], 40).is_none());
    }

    #[test]
    fn shows_intent_todos_and_workers_sections() {
        let mut plan = crate::todo::TodoPlan::default();
        plan.user_intention = Some("Fix mouse selection".to_string());
        let todos = vec![
            todo("Audit flow", "completed"),
            todo("Implement fix", "in_progress"),
            todo("Run tests", "pending"),
        ];
        let workers = vec![
            worker("cargo check", BackgroundTaskRowStatus::Completed),
            worker("compile jcode-tui", BackgroundTaskRowStatus::Running),
            worker("compile test target", BackgroundTaskRowStatus::Failed),
        ];
        let lines = render_panel_lines(&todos, &plan, &workers, 48).expect("panel");
        let text: Vec<String> = lines.iter().map(line_text).collect();
        let joined = text.join("\n");
        assert!(joined.contains("INTENT"), "{joined}");
        assert!(joined.contains("Fix mouse selection"), "{joined}");
        assert!(joined.contains("TODO"), "{joined}");
        assert!(joined.contains("WORKERS"), "{joined}");
        assert!(joined.contains("✓"), "{joined}");
        assert!(joined.contains("✕"), "{joined}");
        assert!(joined.contains("3/3 todo · 1/3 workers"), "{joined}");
        // Running worker stays visible with a spinner frame.
        assert!(joined.contains("bg compile jcode-tui"), "{joined}");
        // Failed worker stays visible.
        assert!(joined.contains("bg compile test target"), "{joined}");
        assert!(joined.contains("failed"), "{joined}");
    }

    #[test]
    fn running_spinner_rotates_through_frames() {
        let frames: Vec<&str> = (0..8).map(|i| running_spinner(i as f32 * 0.25)).collect();
        assert!(frames.iter().any(|f| *f != frames[0]), "spinner must rotate");
        for f in &frames {
            assert!(["◐", "◓", "◑", "◒"].contains(f), "unexpected frame {f}");
        }
    }

    #[test]
    fn many_workers_collapse_into_compact_summary() {
        let mut workers: Vec<crate::tui::BackgroundTaskRow> = (0..7)
            .map(|i| worker(&format!("task {i}"), BackgroundTaskRowStatus::Completed))
            .collect();
        workers.push(worker("running one", BackgroundTaskRowStatus::Running));
        let todos = vec![todo("one", "completed")];
        let plan = crate::todo::TodoPlan::default();
        let lines = render_panel_lines(&todos, &plan, &workers, 48).expect("panel");
        let joined: String = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("+3 more"), "{joined}");
    }

    #[test]
    fn narrow_terminal_returns_none() {
        let todos = vec![todo("one", "pending")];
        let plan = crate::todo::TodoPlan::default();
        assert!(render_panel_lines(&todos, &plan, &[], 12).is_none());
    }
}