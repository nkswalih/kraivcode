// Tests for editor-style mouse drag text selection in the prompt composer
// (input box). Drag-to-select works without Alt+Y, and releasing does NOT
// auto-copy — the selection persists for the user to act on (Ctrl+C copies,
// Backspace/Delete removes, typing replaces).

/// Scan the rendered frame for screen cells that hit-test into the composer
/// (`Input`) pane, returning `(col, row, point)` triples.
fn input_pane_screen_points(
    width: u16,
    height: u16,
) -> Vec<(u16, u16, crate::tui::CopySelectionPoint)> {
    let mut points = Vec::new();
    for row in 0..height {
        for col in 0..width {
            if let Some(point) = crate::tui::ui::copy_point_from_screen(col, row)
                && point.pane == crate::tui::CopySelectionPane::Input
            {
                points.push((col, row, point));
            }
        }
    }
    points
}

/// Find the first screen cell whose CopySelectionPoint matches the given
/// (abs_line, column) pair. Panics if no cell matches.
fn cell_for_point(
    points: &[(u16, u16, crate::tui::CopySelectionPoint)],
    abs_line: usize,
    column: usize,
) -> (u16, u16) {
    points
        .iter()
        .find(|(_, _, p)| p.abs_line == abs_line && p.column == column)
        .map(|(c, r, _)| (*c, *r))
        .unwrap_or_else(|| panic!("no cell for point (line={abs_line}, col={column})"))
}

/// Find the first cell on the given line whose point column is >= `column`.
/// Useful for selecting past the end of text (where all overshoot cells clamp
/// to the line width).
fn cell_at_or_after_point(
    points: &[(u16, u16, crate::tui::CopySelectionPoint)],
    abs_line: usize,
    column: usize,
) -> (u16, u16) {
    points
        .iter()
        .find(|(_, _, p)| p.abs_line == abs_line && p.column >= column)
        .map(|(c, r, _)| (*c, *r))
        .unwrap_or_else(|| panic!("no cell >= point (line={abs_line}, col={column})"))
}

/// Perform a mouse drag (Down → Drag → Up) via `handle_copy_selection_mouse_with`.
/// Returns the persistent selection text AFTER release (no auto-copy).
/// Asserts the copy closure is never called (auto-copy disabled).
fn drag_select(
    app: &mut App,
    start: (u16, u16),
    end: (u16, u16),
) -> String {
    let copy_was_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let copy_was_called_clone = copy_was_called.clone();

    // Down
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: start.0,
            row: start.1,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    // Drag
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: end.0,
            row: end.1,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    // Up — must NOT auto-copy
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: end.0,
            row: end.1,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    assert!(
        !copy_was_called.load(std::sync::atomic::Ordering::SeqCst),
        "auto-copy must NOT fire on drag release (editor-style)"
    );

    // Mode is off but selection (anchor + cursor) persists.
    assert!(!app.copy_selection_mode, "mode must be off after editor-style drag");
    assert!(
        app.copy_selection_anchor.is_some() && app.copy_selection_cursor.is_some(),
        "selection (anchor + cursor) must persist after release"
    );

    app.current_copy_selection_text().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Existing tests, updated to the new editor-style drag behavior
// ---------------------------------------------------------------------------

#[test]
fn test_input_composer_drag_selects_typed_text_without_auto_copy() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "select this draft".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    assert!(!points.is_empty(), "composer must be hit-testable");

let start = cell_for_point(&points, 0, 0);
    let width = unicode_width::UnicodeWidthStr::width("select this draft");
    // Drag to a cell past the text end to select the full line.
    let end = cell_at_or_after_point(&points, 0, width);

    // Kraivcode keeps editor-style drag selection: release selects without
    // copying. Upstream's copy-on-release does not apply to this path.
    let selected = drag_select(&mut app, start, end);
    assert_eq!(selected, "select this draft");
    assert_ne!(app.status_notice(), Some("Copied selection".to_string()));
    assert_ne!(
        app.status_notice(),
        Some("Copied selection · highlight remains visible".to_string())
    );
}

#[test]
fn test_input_composer_selection_never_includes_prompt_prefix() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "no prompt here".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    let rendered = render_and_snap(&app, &mut terminal);
    assert!(rendered.contains("1>"), "expected prompt prefix on screen");

    let points = input_pane_screen_points(80, 24);
    // The leftmost hit-testable cell sits over the prompt decoration (inside
    // the composer content area, before the typed text). Starting the drag
    // there must clamp the selection to the first typed character.
    let (start_col, row, _) = points[0];

    let width = unicode_width::UnicodeWidthStr::width("no prompt here");
    let end = cell_at_or_after_point(&points, 0, width);

    let selected = drag_select(&mut app, (start_col, row), end);
    assert_eq!(selected, "no prompt here");
    assert!(!selected.contains('>'), "prompt must never be copied");
}

#[test]
fn test_input_composer_multiline_selection_preserves_newlines() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "alpha one\nbeta two".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    assert_eq!(crate::tui::ui::input_pane_line_count(), Some(2));

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);
    // Drag to past the end of the second line.
    let width_1 = unicode_width::UnicodeWidthStr::width("beta two");
    let end = cell_at_or_after_point(&points, 1, width_1);

    let selected = drag_select(&mut app, start, end);
    assert_eq!(selected, "alpha one\nbeta two");
}

#[test]
fn test_input_composer_soft_wrapped_selection_copies_unwrapped_text() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    let text = "abcdefghij klmnopqrst uvwxyz0123456789";
    app.input = text.to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(30, 20);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let wrapped_rows = crate::tui::ui::input_pane_line_count().expect("input snapshot");
    assert!(
        wrapped_rows >= 2,
        "expected the input to soft-wrap, got {wrapped_rows} rows"
    );

    let points = input_pane_screen_points(30, 20);
    let start = cell_for_point(&points, 0, 0);
    let last_line = wrapped_rows - 1;
    // Overshoot cells on the last wrapped line clamp to that line's display
    // width, so derive the column from the actual hit-testable cells.
    let last_line_width = points
        .iter()
        .filter(|(_, _, p)| p.abs_line == last_line)
        .map(|(_, _, p)| p.column)
        .max()
        .expect("last wrapped line has hit-testable cells");
    let end = cell_at_or_after_point(&points, last_line, last_line_width);

    let selected = drag_select(&mut app, start, end);
    assert_eq!(selected, text);
    assert!(!selected.contains('\n'));
}

#[test]
fn test_chat_drag_into_composer_clamps_to_chat_pane() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.display_messages = vec![DisplayMessage {
        role: "user".to_string(),
        content: "transcript prompt line".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: None,
        pasted_segments: None,
    }];
    app.bump_display_messages_version();
    app.input = "draft under composition".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let (chat_col, chat_row, chat_point) = (0..24u16)
        .flat_map(|row| (0..80u16).map(move |col| (col, row)))
        .find_map(|(col, row)| {
            crate::tui::ui::copy_point_from_screen(col, row)
                .filter(|p| p.pane == crate::tui::CopySelectionPane::Chat)
                .map(|p| (col, row, p))
        })
        .expect("a chat cell to anchor on");

    let (chat_col2, chat_row2, _) = (0..24u16)
        .flat_map(|row| (0..80u16).map(move |col| (col, row)))
        .find_map(|(col, row)| {
            crate::tui::ui::copy_point_from_screen(col, row)
                .filter(|p| p.pane == crate::tui::CopySelectionPane::Chat && *p != chat_point)
                .map(|p| (col, row, p))
        })
        .expect("a second distinct chat cell");

    let input_points = input_pane_screen_points(80, 24);
    let (input_col, input_row) = input_points
        .iter()
        .map(|(c, r, _)| (*c, *r))
        .next()
        .expect("composer cell");

    let copy_was_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let copy_was_called_clone = copy_was_called.clone();

    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: chat_col,
            row: chat_row,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    // Move within the chat pane, then into the composer.
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: chat_col2,
            row: chat_row2,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: input_col,
            row: input_row,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    // The selection must stay clamped to the chat pane.
    assert_eq!(
        app.current_copy_selection_pane(),
        Some(crate::tui::CopySelectionPane::Chat)
    );

    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: input_col,
            row: input_row,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    // No auto-copy should fire.
    assert!(
        !copy_was_called.load(std::sync::atomic::Ordering::SeqCst),
        "auto-copy must NOT fire on cross-pane chat drag release"
    );

    // The selected text must not contain composer text.
    let selected = app.current_copy_selection_text().unwrap_or_default();
    assert!(
        !selected.contains("draft under composition"),
        "cross-pane drag must not leak composer text, got {selected:?}"
    );
}

#[test]
fn test_input_composer_click_still_moves_caret() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "caret target".to_string();
    app.cursor_pos = 0;

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let (col, row, point) = points
        .iter()
        .find(|(_, _, p)| p.abs_line == 0 && p.column == 6)
        .copied()
        .expect("screen cell inside the typed text");

    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::empty(),
    });
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::empty(),
    });

    assert_eq!(
        app.cursor_pos, point.column,
        "plain click must reposition the caret"
    );
    // No selection was made or copied by the plain click.
    assert!(app.copy_selection_anchor.is_none());
    assert_ne!(app.status_notice(), Some("Copied selection".to_string()));
}

#[test]
fn test_input_composer_drag_release_exits_mode_keeps_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "full path check".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);
    let width = unicode_width::UnicodeWidthStr::width("full path check");
    let end = cell_at_or_after_point(&points, 0, width);

    // Full handle_mouse_event path: press, drag, release.
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: start.0,
        row: start.1,
        modifiers: KeyModifiers::empty(),
    });
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: end.0,
        row: end.1,
        modifiers: KeyModifiers::empty(),
    });
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: end.0,
        row: end.1,
        modifiers: KeyModifiers::empty(),
    });

    // Editor-style: mode is off, selection persists, no copy notice.
    assert!(
        !app.copy_selection_mode,
        "mode must be off after editor-style drag release"
    );
    assert!(
        app.copy_selection_anchor.is_some(),
        "selection anchor must persist"
    );
    assert!(
        app.copy_selection_cursor.is_some(),
        "selection cursor must persist"
    );
    let selected = app.current_copy_selection_text().unwrap_or_default();
    assert_eq!(selected, "full path check");
    assert_ne!(app.status_notice(), Some("Copied selection".to_string()));
    assert_ne!(
        app.status_notice(),
        Some("Copied selection · highlight remains visible".to_string())
    );
    assert_ne!(
        app.status_notice(),
        Some("Failed to copy selection".to_string())
    );
}

// ---------------------------------------------------------------------------
// New regression tests for editor-style selection workflow
// ---------------------------------------------------------------------------

#[test]
fn test_input_composer_drag_then_ctrl_copies_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "select this draft".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);
    let width = unicode_width::UnicodeWidthStr::width("select this draft");
    let end = cell_at_or_after_point(&points, 0, width);

    // Drag to select.
    let _ = drag_select(&mut app, start, end);

    // Ctrl+C should copy the selection.
    let clipboard = CapturedClipboard::new();
    app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL)
        .unwrap();

    assert_eq!(clipboard.text(), Some("select this draft".to_string()));
    assert_eq!(app.status_notice(), Some("Copied selection".to_string()));
    // Ctrl+C also exits copy-selection mode (clears anchor/cursor).
    assert!(app.copy_selection_anchor.is_none());
    assert!(app.copy_selection_cursor.is_none());
}

#[test]
fn test_input_composer_drag_then_backspace_deletes_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "abc XYZ def".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    // "XYZ" spans columns 4..7 (exclusive). Drag from col 4 to col 7.
    let start = cell_for_point(&points, 0, 4);
    let end = cell_for_point(&points, 0, 7);

    let _ = drag_select(&mut app, start, end);

    // Backspace should delete the selection.
    app.handle_key(KeyCode::Backspace, KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input, "abc  def");
    assert_eq!(app.cursor_pos, 4);
    assert!(app.copy_selection_anchor.is_none());
}

#[test]
fn test_input_composer_drag_then_delete_deletes_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "abc XYZ def".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 4);
    let end = cell_for_point(&points, 0, 7);

    let _ = drag_select(&mut app, start, end);

    // Delete should delete the selection.
    app.handle_key(KeyCode::Delete, KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input, "abc  def");
    assert_eq!(app.cursor_pos, 4);
    assert!(app.copy_selection_anchor.is_none());
}

#[test]
fn test_input_composer_drag_then_typing_replaces_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "abc XYZ def".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 4);
    let end = cell_for_point(&points, 0, 7);

    let _ = drag_select(&mut app, start, end);

    // Typing should replace the selected text.
    app.handle_key(KeyCode::Char('Q'), KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input, "abc Q def");
    assert_eq!(app.cursor_pos, 5);
    assert!(app.copy_selection_anchor.is_none());
}

#[test]
fn test_input_composer_drag_then_arrow_clears_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "select this draft".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);
    let width = unicode_width::UnicodeWidthStr::width("select this draft");
    let end = cell_at_or_after_point(&points, 0, width);

    let _ = drag_select(&mut app, start, end);

    // Left arrow should clear the selection and move the cursor.
    let prev_cursor = app.cursor_pos;
    app.handle_key(KeyCode::Left, KeyModifiers::empty())
        .unwrap();

    assert!(
        app.copy_selection_anchor.is_none(),
        "arrow must clear selection anchor"
    );
    assert!(app.copy_selection_cursor.is_none());
    assert_eq!(app.cursor_pos, prev_cursor.saturating_sub(1));
}

#[test]
fn test_input_composer_drag_select_all_then_backspace_clears_input() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "everything".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);
    let width = unicode_width::UnicodeWidthStr::width("everything");
    let end = cell_at_or_after_point(&points, 0, width);

    let _ = drag_select(&mut app, start, end);

    app.handle_key(KeyCode::Backspace, KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input, "");
    assert_eq!(app.cursor_pos, 0);
    assert!(app.copy_selection_anchor.is_none());
}

#[test]
fn test_input_composer_second_drag_replaces_first_selection() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "abc XYZ def".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);

    // First drag: select "abc" (cols 0..3).
    let start1 = cell_for_point(&points, 0, 0);
    let end1 = cell_for_point(&points, 0, 3);
    let selected1 = drag_select(&mut app, start1, end1);
    assert_eq!(selected1, "abc");

    // Second drag: select "def" (cols 8..11).
    let start2 = cell_for_point(&points, 0, 8);
    let end2 = cell_for_point(&points, 0, 11);
    let selected2 = drag_select(&mut app, start2, end2);
    assert_eq!(selected2, "def");

    // Verify no stale selection artifacts.
    assert!(!app.copy_selection_mode);
    let text = app.current_copy_selection_text().unwrap_or_default();
    assert_eq!(text, "def");
}

#[test]
fn test_input_composer_drag_on_empty_input_selects_nothing() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "".to_string();
    app.cursor_pos = 0;

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    if points.is_empty() {
        // Empty input may not produce any hit-testable cells.
        return;
    }

    let (col, row, _) = points[0];
    let copy_was_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let copy_was_called_clone = copy_was_called.clone();

    // Down
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );
    // Drag (same cell or adjacent)
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: col + 1,
            row,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: col + 1,
            row,
            modifiers: KeyModifiers::empty(),
        },
        |_| {
            copy_was_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        },
    );

    assert!(
        !copy_was_called.load(std::sync::atomic::Ordering::SeqCst),
        "auto-copy must NOT fire on drag over empty input"
    );
    // No selection should be present.
    assert!(app.copy_selection_anchor.is_none() || {
        app.current_copy_selection_text()
            .map_or(true, |t| t.is_empty())
    });
}

#[test]
fn test_input_composer_drag_to_cell_past_end_selects_full_text() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "short".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);

    // A cell at/after the text end (width=5) clamps to column 5.
    let width = unicode_width::UnicodeWidthStr::width("short");
    let end = cell_at_or_after_point(&points, 0, width);

    let selected = drag_select(&mut app, start, end);
    assert_eq!(selected, "short");
}

#[test]
fn test_input_composer_drag_via_moved_fallback_when_terminal_sends_no_drag() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.input = "select this draft".to_string();
    app.cursor_pos = app.input.len();

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let points = input_pane_screen_points(80, 24);
    let start = cell_for_point(&points, 0, 0);
    let width = unicode_width::UnicodeWidthStr::width("select this draft");
    let end = cell_at_or_after_point(&points, 0, width);

    // Simulate a terminal that sends Moved instead of Drag during button-hold
    // (e.g. Windows ConPTY with button state lost in translation).
    // Down (plain) → arms pending anchor.
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: start.0,
            row: start.1,
            modifiers: KeyModifiers::empty(),
        },
        |_| true,
    );
    assert!(app.copy_selection_pending_anchor.is_some(), "pending anchor must be set on plain Down");

    // Moved → the fallback handler should auto-enter mode and start the selection.
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Moved,
            column: end.0,
            row: end.1,
            modifiers: KeyModifiers::empty(),
        },
        |_| true,
    );
    assert!(app.copy_selection_mode, "Moved fallback must enter mode");
    assert!(app.copy_selection_dragging, "Moved fallback must set dragging");
    assert!(app.copy_selection_anchor.is_some(), "Moved fallback must set anchor");
    assert!(app.copy_selection_cursor.is_some(), "Moved fallback must set cursor");

    // Up → exits mode, keeps selection, no auto-copy.
    app.handle_copy_selection_mouse_with(
        MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: end.0,
            row: end.1,
            modifiers: KeyModifiers::empty(),
        },
        |_| true,
    );
    assert!(!app.copy_selection_mode, "mode must be off after editor-style release");
    assert!(app.copy_selection_anchor.is_some(), "selection must persist");
    let selected = app.current_copy_selection_text().unwrap_or_default();
    assert_eq!(selected, "select this draft");
}