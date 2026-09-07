//! Shared copy-selection highlight rendering.
//!
//! One implementation for every pane that supports drag-to-select (chat
//! viewport, side pane, file-diff pane, full-screen overlays, and the prompt
//! composer) so the selection style stays visually identical across the UI.

use super::{accent_color, blend_color, rgb};
use ratatui::prelude::*;

pub(crate) fn selection_bg_for(base_bg: Option<Color>) -> Color {
    let fallback = rgb(32, 38, 48);
    blend_color(base_bg.unwrap_or(fallback), accent_color(), 0.34)
}

pub(crate) fn selection_fg_for(base_fg: Option<Color>) -> Option<Color> {
    base_fg.map(|fg| blend_color(fg, Color::White, 0.15))
}

/// Apply a copy-selection highlight to a single display line between
/// `[start_col, end_col)` (display columns). Zero-width characters inherit the
/// selection of the cell they attach to; wide characters are selected when any
/// of their cells overlap the range.
pub(crate) fn highlight_line_selection(
    line: &Line<'static>,
    start_col: usize,
    end_col: usize,
) -> Line<'static> {
    if end_col <= start_col {
        return line.clone();
    }

    let mut rebuilt: Vec<Span<'static>> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;
    let mut col = 0usize;
    // Whether the most recently placed *cell* (i.e. the last non-zero-width
    // character) was selected. Zero-width characters (combining marks,
    // joiners) attach to that preceding cell and must inherit its selected
    // state exactly.
    //
    // This can't be recomputed from `col` the same way normal cells are:
    // a zero-width char's `col` equals the *end* boundary of the preceding
    // cell (col_prev + width_prev), whereas the preceding cell itself only
    // needs to *overlap* `[start_col, end_col)` to count as selected. Reusing
    // the overlap test with that end-boundary `col` silently requires the
    // whole preceding cell to fit inside the range, so a selection edge
    // landing mid-cell would highlight the base glyph but not its attached
    // combining mark, splitting the highlight. Tracking and reusing the
    // preceding cell's own selected flag keeps the two in lockstep.
    let mut last_cell_selected = false;

    let flush = |rebuilt: &mut Vec<Span<'static>>, text: &mut String, style: &mut Option<Style>| {
        if !text.is_empty() {
            let span = match style.take() {
                Some(style) => Span::styled(std::mem::take(text), style),
                None => Span::raw(std::mem::take(text)),
            };
            rebuilt.push(span);
        }
    };

    for span in &line.spans {
        let mut selected_style = span.style.bg(selection_bg_for(span.style.bg));
        if let Some(fg) = selection_fg_for(span.style.fg) {
            selected_style = selected_style.fg(fg);
        }
        for ch in span.content.chars() {
            let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            let selected = if width == 0 {
                last_cell_selected
            } else {
                let is_selected = col < end_col && col.saturating_add(width) > start_col;
                last_cell_selected = is_selected;
                is_selected
            };

            let style = if selected { selected_style } else { span.style };

            if current_style == Some(style) {
                current_text.push(ch);
            } else {
                flush(&mut rebuilt, &mut current_text, &mut current_style);
                current_text.push(ch);
                current_style = Some(style);
            }

            col = col.saturating_add(width);
        }
    }

    flush(&mut rebuilt, &mut current_text, &mut current_style);

    Line {
        spans: rebuilt,
        style: line.style,
        alignment: line.alignment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_width_char_matches_preceding_wide_cell_when_boundary_splits_it() {
        // A 2-wide glyph (col 0..2) with a zero-width combining mark
        // attached, selection ending mid-glyph at col 1: the wide glyph is
        // selected (any overlap), so the combining mark attached to it must
        // be selected too, not left out because its own end boundary (2)
        // exceeds end_col (1).
        let line = Line::from(vec![Span::raw("\u{4e2d}\u{0301}")]); // 中 + combining acute
        let highlighted = highlight_line_selection(&line, 0, 1);
        // Every char must carry the selection background; none should still
        // have the original (unset) background.
        for span in &highlighted.spans {
            assert!(
                span.style.bg.is_some(),
                "expected selected background on {:?}",
                span.content
            );
        }
    }

    #[test]
    fn zero_width_char_stays_unselected_with_preceding_cell() {
        let line = Line::from(vec![Span::raw("\u{4e2d}\u{0301}ab")]);
        // Selection covers only "ab" (cols 2..4); the leading wide glyph and
        // its combining mark must both stay unselected.
        let highlighted = highlight_line_selection(&line, 2, 4);
        let rebuilt_text: String = highlighted
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(rebuilt_text, "\u{4e2d}\u{0301}ab");
        // First span (the unselected prefix) must have no selection bg.
        assert!(highlighted.spans[0].style.bg.is_none());
    }

    #[test]
    fn empty_selection_returns_line_unchanged() {
        let line = Line::from(vec![Span::raw("hello")]);
        let highlighted = highlight_line_selection(&line, 3, 3);
        assert_eq!(highlighted.spans.len(), line.spans.len());
        assert!(highlighted.spans[0].style.bg.is_none());
    }
}