use super::{
    accent_color, ai_color, ai_text, asap_color, clear_area, dim_color, get_grouped_changelog,
    header_icon_color, header_name_color, header_session_color, pending_color, queued_color,
    record_chat_overlay_copy_snapshot, render_rounded_box, rgb, tool_color, user_bg, user_color,
    user_text,
};
use std::sync::{Mutex, OnceLock};
use crate::tui::TuiState;
use crate::tui::info_widget::WidgetPlacement;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use super::selection_highlight::highlight_line_selection;

// ── Ask-user popup palette (matches skills picker theme) ──────────────
const PANEL_BG: Color = Color::Rgb(16, 16, 14);
const PANEL_BORDER: Color = Color::Rgb(84, 64, 40);
const SELECTED_BG: Color = Color::Rgb(46, 32, 16);
const MUTED: Color = Color::Rgb(172, 152, 112);
const MUTED_DARK: Color = Color::Rgb(118, 100, 74);
const ACCENT: Color = Color::Rgb(255, 140, 0);
const HINT: Color = Color::Rgb(170, 210, 255);

fn hotkey(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::default().fg(Color::White).bg(Color::DarkGray))
}

pub(super) fn draw_changelog_overlay(
    frame: &mut Frame,
    area: Rect,
    scroll: usize,
    app: &dyn TuiState,
) {
    clear_area(frame, area);

    let groups = get_grouped_changelog();
    let mut lines: Vec<Line<'static>> = Vec::new();

    if groups.is_empty() {
        lines.push(Line::from(Span::styled(
            "No changelog entries available.",
            Style::default().fg(dim_color()),
        )));
    } else {
        for group in &groups {
            let heading = match &group.released_at {
                Some(released_at) => format!("  {} · {}", group.version, released_at),
                None => format!("  {}", group.version),
            };
            lines.push(Line::from(Span::styled(
                heading,
                Style::default()
                    .fg(rgb(200, 200, 220))
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for entry in &group.entries {
                lines.push(Line::from(vec![
                    Span::styled("    • ", Style::default().fg(dim_color())),
                    Span::styled(entry.clone(), Style::default().fg(rgb(170, 170, 185))),
                ]));
            }
            lines.push(Line::from(""));
        }
    }

    let total_lines = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll.min(max_scroll);

    let scroll_info = if total_lines > visible_height {
        let pct = if max_scroll > 0 {
            (scroll * 100) / max_scroll
        } else {
            100
        };
        format!(" {}% ", pct)
    } else {
        String::new()
    };

    let title = format!(" Changelog {} ", scroll_info);
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(rgb(200, 200, 220))
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " Esc to close · drag to select, release to copy · wheel/j/k scroll ",
            Style::default().fg(dim_color()),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dim_color()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let visible_end = scroll
        .saturating_add(inner.height as usize)
        .min(total_lines);

    // Register the rendered lines so the shared copy-selection machinery can map
    // mouse drags to text and highlight + copy the selection, exactly like the
    // chat viewport. Without this, mouse capture would block native terminal
    // selection and there would be no way to copy from the overlay.
    record_chat_overlay_copy_snapshot(&lines, scroll, visible_end, inner);

    let mut visible_lines: Vec<Line<'static>> =
        lines.get(scroll..visible_end).unwrap_or(&[]).to_vec();

    if let Some(range) = app.copy_selection_range().filter(|range| {
        range.start.pane == crate::tui::CopySelectionPane::Chat
            && range.end.pane == crate::tui::CopySelectionPane::Chat
    }) {
        let (start, end) = if (range.start.abs_line, range.start.column)
            <= (range.end.abs_line, range.end.column)
        {
            (range.start, range.end)
        } else {
            (range.end, range.start)
        };
        for abs_idx in start.abs_line.max(scroll)..=end.abs_line.min(visible_end.saturating_sub(1))
        {
            let rel_idx = abs_idx.saturating_sub(scroll);
            if let Some(line) = visible_lines.get_mut(rel_idx) {
                let start_col = if abs_idx == start.abs_line {
                    start.column
                } else {
                    0
                };
                let end_col = if abs_idx == end.abs_line {
                    end.column
                } else {
                    line.width()
                };
                *line = highlight_line_selection(line, start_col, end_col);
            }
        }
    }

    frame.render_widget(Paragraph::new(visible_lines), inner);
}

pub(super) fn draw_help_overlay(frame: &mut Frame, area: Rect, scroll: usize, app: &dyn TuiState) {
    clear_area(frame, area);

    let section_style = Style::default()
        .fg(accent_color())
        .add_modifier(Modifier::BOLD);
    let cmd_style = Style::default().fg(rgb(230, 230, 240));
    let desc_style = Style::default().fg(rgb(150, 150, 165));
    let key_style = Style::default().fg(rgb(200, 180, 120));
    let sep_style = Style::default().fg(rgb(50, 50, 55));

    let mut lines: Vec<Line<'static>> = Vec::new();

    let separator = || -> Line<'static> {
        Line::from(Span::styled(
            "  ─────────────────────────────────────────────────",
            sep_style,
        ))
    };

    let help_entry = |cmd: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(cmd.to_string(), cmd_style),
            Span::styled("  ", Style::default()),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    let alt = jcode_tui_core::keybind::alt_chord;
    let key_entry = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(format!("{:<22}", key), key_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Commands", section_style)));
    lines.push(Line::from(""));
    lines.push(help_entry("/help", "Show this help overlay"));
    lines.push(help_entry(
        "/help <command>",
        "Show details for one command",
    ));
    lines.push(help_entry("/model", "List or switch models"));
    lines.push(help_entry("/model <name>", "Switch to a different model"));
    lines.push(help_entry(
        "/provider-test-coverage",
        "Show live-test evidence for the current provider/model",
    ));
    lines.push(help_entry("/agents", "Configure models for agent roles"));
    lines.push(help_entry(
        "/swarm-prompt",
        "Open the active swarm routing prompt in your editor",
    ));
    lines.push(help_entry(
        "/effort <level>",
        "Set effort (none|minimal|low|medium|high|xhigh|max|swarm|swarm-deep)",
    ));
    lines.push(help_entry(
        "/fast [on|off|status|default ...]",
        "Toggle fast mode",
    ));
    lines.push(help_entry(
        "/transport <mode>",
        "Set connection transport (auto|https|websocket)",
    ));
    lines.push(help_entry(
        "/alignment [status|centered|left]",
        "Show or persist text alignment preference",
    ));
    lines.push(help_entry(
        "/compact-notifications [status|on|off]",
        "Collapse swarm/file-activity notifications to one line",
    ));
    lines.push(help_entry(
        "/show-agentgrep-output [status|on|off]",
        "Render full agentgrep search output inline in chat",
    ));
    lines.push(help_entry("/config", "Show active configuration"));
    lines.push(help_entry("/config init", "Create default config file"));
    lines.push(help_entry("/config edit", "Open config in $EDITOR"));
    lines.push(help_entry("/dictate", "Run configured external dictation"));
    lines.push(help_entry(
        "/git [status]",
        "Show branch and working tree status for the repo",
    ));
    lines.push(help_entry(
        "/context",
        "Show the full session context snapshot",
    ));
    lines.push(help_entry(
        "/skills",
        "Show loaded skills and jcode-endorsed recommendations",
    ));
    lines.push(help_entry("/info", "Show session info and token usage"));
    lines.push(help_entry(
        "/keys",
        "Show keybinding conflicts with your terminal/OS",
    ));
    lines.push(help_entry("/usage", "Show connected provider usage limits"));
    lines.push(help_entry(
        "/support",
        "Email support with diagnostics prefilled",
    ));
    lines.push(help_entry("/version", "Show version and build details"));
    lines.push(help_entry(
        "/changelog",
        "Show recent changes in this build",
    ));

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Session", section_style)));
    lines.push(Line::from(""));
    lines.push(help_entry("/clear", "Clear conversation and start fresh"));
    lines.push(help_entry(
        "/compact",
        "Summarize old messages to free context",
    ));
    lines.push(help_entry(
        "/rewind",
        "Show numbered history, /rewind N to rewind",
    ));
    lines.push(help_entry(
        "/fix",
        "Attempt recovery when model cannot continue",
    ));
    lines.push(help_entry(
        "/poke",
        "Poke model to resume with incomplete todos (on/off/status)",
    ));
    lines.push(help_entry(
        "/plan [goal]",
        "Draft a plan-only proposal as a plan card (no edits)",
    ));
    lines.push(help_entry(
        "/improve",
        "Autonomously improve the repo until returns diminish",
    ));
    lines.push(help_entry(
        "/improve resume",
        "Resume the last saved improve loop/plan",
    ));
    lines.push(help_entry(
        "/refactor",
        "Run a safe refactor loop with independent review",
    ));
    lines.push(help_entry(
        "/refactor resume",
        "Resume the last saved refactor loop/plan",
    ));
    lines.push(help_entry(
        "/splitview [on|off|status]",
        "Mirror the current chat in the side panel",
    ));
    lines.push(help_entry(
        "/fork [prompt]",
        "Fork session into a new window (alias: /split)",
    ));
    lines.push(help_entry(
        "/transfer",
        "Open a fresh session with only compacted context + copied todos",
    ));
    lines.push(help_entry(
        "/workspace [status|on|off|add]",
        "Enable and manage the Niri-style session workspace",
    ));
    lines.push(help_entry(
        "/catchup [next|list]",
        "Jump to finished sessions and open a Catch Up brief",
    ));
    lines.push(help_entry(
        "/back",
        "Return to the previous Catch Up source session",
    ));
    lines.push(help_entry("/resume", "Browse and resume previous sessions"));
    lines.push(help_entry(
        "/active",
        "Manage live sessions: see which are working vs ready",
    ));
    lines.push(help_entry(
        "/catchup [next]",
        "Jump into finished sessions with a side-panel brief",
    ));
    lines.push(help_entry(
        "/back",
        "Return to the previous Catch Up session",
    ));
    lines.push(help_entry("/save [label]", "Bookmark session for /resume"));
    lines.push(help_entry(
        "/rename <name>|--clear",
        "Set or clear current session name",
    ));
    lines.push(help_entry(
        "/unsave",
        "Remove bookmark from current session",
    ));

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Memory & Swarm", section_style)));
    lines.push(Line::from(""));
    lines.push(help_entry("/memory [on|off]", "Toggle memory features"));
    lines.push(help_entry(
        "/test [claim]",
        "Run layered verification and produce proof",
    ));
    lines.push(help_entry(
        "/initiatives",
        "Open initiatives overview / resume an initiative",
    ));
    lines.push(help_entry("/swarm [on|off]", "Toggle swarm features"));

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Auth & Accounts", section_style)));
    lines.push(Line::from(""));
    lines.push(help_entry("/auth", "Show authentication status"));
    lines.push(help_entry(
        "/login [provider]",
        "Interactive or direct login",
    ));
    lines.push(help_entry(
        "/account",
        "Open combined Claude/OpenAI account picker",
    ));
    lines.push(help_entry(
        "/subscription",
        "Inspect jcode subscription scaffold",
    ));
    lines.push(help_entry(
        "/subscribe",
        "Why and how to subscribe to jcode",
    ));

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  System", section_style)));
    lines.push(Line::from(""));
    lines.push(help_entry("/reload", "Reload to newer binary if available"));
    lines.push(help_entry(
        "/restart",
        "Restart with current binary (no build)",
    ));
    lines.push(help_entry(
        "/rebuild",
        "Full update (git pull + build + tests)",
    ));
    if app.is_remote_mode() {
        lines.push(help_entry("/client-reload", "Force reload client binary"));
        lines.push(help_entry("/server-reload", "Force reload server binary"));
        lines.push(help_entry(
            "/continue",
            "Continue every interrupted live session that would auto-resume",
        ));
    }
    lines.push(help_entry(
        "/debug-visual",
        "Enable visual debugging for TUI issues",
    ));
    lines.push(help_entry("/quit", "Exit jcode"));

    // The sections above are hand-curated for ordering, but they drift as
    // commands are added. Anything registered and not already shown gets listed
    // here so no working command is invisible in /help.
    let shown: std::collections::HashSet<String> = lines
        .iter()
        .filter_map(|line| line.spans.get(1).map(|span| span.content.to_string()))
        .map(|cmd| {
            cmd.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    let uncovered: Vec<(&str, &str)> = crate::tui::app::registered_command_entries()
        .filter(|(name, _)| !shown.contains(*name))
        .collect();
    if !uncovered.is_empty() {
        lines.push(Line::from(""));
        lines.push(separator());
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  More commands", section_style)));
        lines.push(Line::from(""));
        for (name, desc) in uncovered {
            lines.push(help_entry(name, desc));
        }
    }

    let skills = app.available_skills();
    if !skills.is_empty() {
        lines.push(Line::from(""));
        lines.push(separator());
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled("  Skills", section_style)));
        lines.push(Line::from(""));
        for skill in &skills {
            lines.push(help_entry(&format!("/{}", skill), "Activate skill"));
        }
    }

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Navigation", section_style)));
    lines.push(Line::from(""));
    lines.push(key_entry("PageUp / PageDown", "Scroll history"));
    lines.push(key_entry("Up / Down", "Scroll history (when input empty)"));
    lines.push(key_entry(
        "Ctrl+J / Ctrl+K",
        "Jump to next / previous user prompt (also Ctrl+] / Ctrl+[)",
    ));
    lines.push(key_entry(
        "Ctrl+Shift+J / Ctrl+Shift+K",
        "Scroll history down / up one line",
    ));
    lines.push(key_entry(
        "Cmd/Super+K / J",
        "Jump to previous / next user prompt (macOS, if forwarded)",
    ));
    lines.push(key_entry("Ctrl+1..4", "Resize side panel to 25/50/75/100%"));
    lines.push(key_entry(
        "Ctrl+5..9",
        "Jump by recency (5 = 5th most recent)",
    ));

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "  Diagrams & Diffs",
        section_style,
    )));
    lines.push(Line::from(""));
    lines.push(key_entry(
        &crate::tui::keybind::side_panel_toggle_key_label(),
        "Toggle side panel (or diagram pane if empty)",
    ));
    lines.push(key_entry(&alt("T"), "Toggle diagram position (side/top)"));
    lines.push(key_entry(
        &alt("Shift+I"),
        "Show/hide inline images (persists)",
    ));
    lines.push(key_entry("Ctrl+H / Ctrl+L", "Focus chat / diagram / diffs"));
    lines.push(key_entry(
        "Ctrl+L / Cmd+L",
        "Clear screen, history stays above (no pane focused)",
    ));
    lines.push(key_entry(
        "Ctrl+Left / Right",
        "Cycle diagrams (when diagram focused)",
    ));
    lines.push(key_entry("h/j/k/l / arrows", "Pan diagram (when focused)"));
    lines.push(key_entry("[ / ]", "Zoom diagram (when focused)"));
    lines.push(key_entry("+ / -", "Resize diagram pane"));
    lines.push(key_entry(
        &format!("{} / /diff", alt("G")),
        "Cycle diff mode (Off/Inline/Pinned/File)",
    ));
    lines.push(key_entry("Shift+Tab", "Cycle favorited models"));
    lines.push(key_entry("Ctrl+O", "Set default model (in /model picker)"));
    lines.push(key_entry(
        "Ctrl+N",
        "Toggle favorite model (in /model picker)",
    ));

    lines.push(Line::from(""));
    lines.push(separator());
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Input & Editing", section_style)));
    lines.push(Line::from(""));
    lines.push(key_entry(
        "Ctrl+C / Ctrl+D",
        "Quit (press twice to confirm)",
    ));
    lines.push(key_entry("Ctrl+X", "Cut entire input line to clipboard"));
    lines.push(key_entry(
        "Ctrl+A",
        "Copy visible chat viewport plus nearby context",
    ));
    lines.push(key_entry("Ctrl+U", "Clear input line"));
    lines.push(key_entry("Ctrl+K", "Delete to end of input"));
    lines.push(key_entry(
        &format!("{} / {}", alt("Backspace"), alt("Delete")),
        "Delete previous word in input",
    ));
    lines.push(key_entry(
        "Cmd/Super+Backspace / Delete",
        "Delete previous word in input",
    ));
    if cfg!(target_os = "macos") {
        // On macOS, Cmd+Left/Right default to effort cycling; Home/End and
        // Cmd+A/E still jump to the start/end of the input.
        lines.push(key_entry("Home / End", "Move to start / end of input"));
    } else {
        lines.push(key_entry(
            "Cmd/Super+Left / Right",
            "Move to start / end of input",
        ));
    }
    lines.push(key_entry("Cmd/Super+Z", "Undo input edit"));
    lines.push(key_entry("Cmd/Super+X / V", "Cut input / paste clipboard"));
    lines.push(key_entry("Ctrl+S", "Stash / pop input (save for later)"));
    lines.push(key_entry("Ctrl+Backspace", "Delete previous word in input"));
    lines.push(key_entry("Ctrl+B / Ctrl+F", "Move by word left / right"));
    lines.push(key_entry("Ctrl+Left / Right", "Move by word left / right"));
    lines.push(key_entry(
        &format!("Shift+Enter / {}", alt("Enter")),
        "Insert newline in input",
    ));
    lines.push(key_entry(
        "Trailing \\ then Enter",
        "Insert newline (fallback; run /terminal-setup to fix Shift+Enter)",
    ));
    lines.push(key_entry(
        "Ctrl+Enter / Cmd+Enter",
        "Use opposite send mode while processing",
    ));
    lines.push(key_entry("Ctrl+Up", "Retrieve pending message for editing"));
    lines.push(key_entry("Ctrl+Tab / Ctrl+T", "Toggle queue mode"));
    lines.push(key_entry(
        "Ctrl+R",
        "Search prompt history (across sessions)",
    ));
    lines.push(key_entry(
        &format!("Ctrl+V / {}", alt("V")),
        "Paste clipboard (text or image)",
    ));
    lines.push(key_entry(
        &alt("A"),
        "Quick-copy visible chat viewport plus nearby context",
    ));
    lines.push(key_entry(&alt("Y"), "Toggle chat selection/copy mode"));
    lines.push(key_entry(&alt("S"), "Toggle typing scroll lock"));
    lines.push(key_entry("Ctrl+P", "Toggle auto-poke for incomplete todos"));
    lines.push(key_entry(&alt("X"), "Show/dismiss todo list card in chat"));
    lines.push(key_entry(
        &crate::tui::keybind::effort_switch_keys_label(),
        "Cycle effort (reasoning + swarm)",
    ));
    if cfg!(target_os = "macos") {
        lines.push(key_entry(
            &alt("Left / Right"),
            &format!("Move by word in input (also {} / {})", alt("B"), alt("F")),
        ));
    }
    if let Some(label) = app.dictation_key_label() {
        lines.push(key_entry(&label, "Run configured dictation"));
    }
    if let Some(label) = crate::tui::keybind::load_open_resume_key().label {
        lines.push(key_entry(&label, "Open the /resume session picker"));
    }
    if let Some(label) = crate::tui::keybind::load_new_terminal_key().label {
        lines.push(key_entry(
            &label,
            "Spawn new jcode session in a new terminal",
        ));
    }

    lines.push(Line::from(""));

    let total_lines = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll.min(max_scroll);

    let scroll_info = if total_lines > visible_height {
        let pct = if max_scroll > 0 {
            (scroll * 100) / max_scroll
        } else {
            100
        };
        format!(" {}% ", pct)
    } else {
        String::new()
    };

    let title = format!(" Help {} ", scroll_info);
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(rgb(200, 200, 220))
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " Esc to close · mouse wheel/j/k scroll · Space/PageUp page · /help <cmd> for details ",
            Style::default().fg(dim_color()),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dim_color()));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

pub(super) fn draw_model_status_overlay(
    frame: &mut Frame,
    area: Rect,
    scroll: usize,
    content: &str,
) {
    clear_area(frame, area);

    let title_style = Style::default()
        .fg(accent_color())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(rgb(210, 210, 220));
    let dim_style = Style::default().fg(dim_color());

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled("  Model Status", title_style)));
    lines.push(Line::from(Span::styled(
        "  Live verification evidence for provider/model behavior in jcode",
        dim_style,
    )));
    lines.push(Line::from(""));

    for raw in content.lines() {
        if let Some(title) = raw.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(format!("  {title}"), title_style)));
        } else if let Some(title) = raw.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(format!("  {title}"), title_style)));
        } else if raw.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  {raw}"),
                model_status_line_style(raw, text_style),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑/↓ scroll, PgUp/PgDn page, c copy report, q/Esc close",
        dim_style,
    )));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" /provider-test-coverage "),
        )
        .scroll((scroll.min(u16::MAX as usize) as u16, 0));
    frame.render_widget(paragraph, area);
}

fn model_status_line_style(raw: &str, default: Style) -> Style {
    // Reuse the same semantic classification the CLI uses so the TUI overlay
    // and `jcode provider-test-coverage` stay color-consistent.
    use crate::live_tests::CoverageLineStyle;
    match crate::live_tests::classify_provider_test_coverage_line(raw) {
        CoverageLineStyle::Title => Style::default()
            .fg(accent_color())
            .add_modifier(Modifier::BOLD),
        CoverageLineStyle::Pass => Style::default().fg(rgb(120, 220, 150)),
        CoverageLineStyle::Fail => Style::default().fg(rgb(240, 110, 110)),
        CoverageLineStyle::Warn => Style::default().fg(rgb(235, 190, 105)),
        CoverageLineStyle::Dim => Style::default().fg(dim_color()),
        CoverageLineStyle::Plain => default,
    }
}

pub(super) fn draw_debug_overlay(
    frame: &mut Frame,
    placements: &[WidgetPlacement],
    chunks: &[Rect],
) {
    if chunks.len() < 5 {
        return;
    }
    render_overlay_box(frame, chunks[0], "messages", Color::Red);
    render_overlay_box(frame, chunks[1], "queued", Color::Yellow);
    render_overlay_box(frame, chunks[2], "swarm", Color::Cyan);
    render_overlay_box(frame, chunks[3], "notification", Color::Magenta);
    render_overlay_box(frame, chunks[4], "inline", Color::Green);
    if chunks.len() > 6 && chunks[6].height > 0 {
        render_overlay_box(frame, chunks[6], "activity", Color::Cyan);
    }
    if chunks.len() > 7 && chunks[7].height > 0 {
        render_overlay_box(frame, chunks[7], "input", Color::Blue);
    }
    if chunks.len() > 8 && chunks[8].height > 0 {
        render_overlay_box(frame, chunks[8], "status", Color::Yellow);
    }
    if chunks.len() > 10 && chunks[10].height > 0 {
        render_overlay_box(frame, chunks[10], "donut", Color::Blue);
    }

    for placement in placements {
        let title = format!("widget:{}", placement.kind.as_str());
        render_overlay_box(frame, placement.rect, &title, Color::Magenta);
    }
}

fn render_overlay_box(frame: &mut Frame, area: Rect, title: &str, color: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(title.to_string(), Style::default().fg(color)));
    frame.render_widget(block, area);
}

pub(super) fn debug_palette_json() -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "user_color": color_to_rgb(user_color()),
        "ai_color": color_to_rgb(ai_color()),
        "tool_color": color_to_rgb(tool_color()),
        "dim_color": color_to_rgb(dim_color()),
        "accent_color": color_to_rgb(accent_color()),
        "queued_color": color_to_rgb(queued_color()),
        "asap_color": color_to_rgb(asap_color()),
        "pending_color": color_to_rgb(pending_color()),
        "user_text": color_to_rgb(user_text()),
        "user_bg": color_to_rgb(user_bg()),
        "ai_text": color_to_rgb(ai_text()),
        "header_icon_color": color_to_rgb(header_icon_color()),
        "header_name_color": color_to_rgb(header_name_color()),
        "header_session_color": color_to_rgb(header_session_color()),
    }))
}

fn color_to_rgb(color: Color) -> Option<[u8; 3]> {
    match color {
        Color::Rgb(r, g, b) => Some([r, g, b]),
        Color::Indexed(n) if n >= 16 => {
            let (r, g, b) = crate::tui::color_support::indexed_to_rgb(n);
            Some([r, g, b])
        }
        _ => None,
    }
}

/// Captured hit-test geometry of the floating model-detail card. Refreshed
/// every frame by the renderer; consumed by mouse routing.
#[derive(Debug, Clone)]
pub(crate) struct ModelDetailPopupGeometry {
    pub card: Rect,
    pub buttons: Vec<(Rect, crate::tui::ModelDetailButton)>,
}

#[cfg(not(test))]
static MODEL_DETAIL_POPUP_GEOMETRY: OnceLock<Mutex<Option<ModelDetailPopupGeometry>>> =
    OnceLock::new();

#[cfg(not(test))]
fn model_detail_popup_geometry_slot()
-> &'static Mutex<Option<ModelDetailPopupGeometry>> {
    MODEL_DETAIL_POPUP_GEOMETRY.get_or_init(|| Mutex::new(None))
}

pub(crate) fn store_model_detail_popup_geometry(geometry: ModelDetailPopupGeometry) {
    #[cfg(not(test))]
    if let Ok(mut slot) = model_detail_popup_geometry_slot().lock() {
        *slot = Some(geometry);
    }
    #[cfg(test)]
    let _ = geometry;
}

pub(crate) fn model_detail_popup_geometry() -> Option<ModelDetailPopupGeometry> {
    #[cfg(not(test))]
    {
        let guard = model_detail_popup_geometry_slot().lock().ok()?;
        guard.clone()
    }
    #[cfg(test)]
    {
        None
    }
}

/// Forget recorded pill/card rects. Must run whenever the popup opens or
/// closes — stale rects made later clicks silently fire "Set as default".
pub(crate) fn clear_model_detail_popup_geometry() {
    #[cfg(not(test))]
    if let Ok(mut slot) = model_detail_popup_geometry_slot().lock() {
        *slot = None;
    }
    #[cfg(test)]
    {}
}

/// Captured hit-test geometry of the permission panel card + pill lines.
#[derive(Debug, Clone)]
pub(crate) struct PermissionPanelGeometry {
    pub card: Rect,
    pub pills: Vec<(Rect, crate::tui::PermissionPanelDecision)>,
}

#[cfg(not(test))]
static PERMISSION_PANEL_GEOMETRY: OnceLock<Mutex<Option<PermissionPanelGeometry>>> =
    OnceLock::new();

#[cfg(not(test))]
fn permission_panel_geometry_slot() -> &'static Mutex<Option<PermissionPanelGeometry>> {
    PERMISSION_PANEL_GEOMETRY.get_or_init(|| Mutex::new(None))
}

pub(crate) fn store_permission_panel_geometry(geometry: PermissionPanelGeometry) {
    #[cfg(not(test))]
    if let Ok(mut slot) = permission_panel_geometry_slot().lock() {
        *slot = Some(geometry);
    }
    #[cfg(test)]
    let _ = geometry;
}

pub(crate) fn permission_panel_geometry() -> Option<PermissionPanelGeometry> {
    #[cfg(not(test))]
    {
        let guard = permission_panel_geometry_slot().lock().ok()?;
        guard.clone()
    }
    #[cfg(test)]
    {
        None
    }
}

pub(crate) fn clear_permission_panel_geometry() {
    #[cfg(not(test))]
    if let Ok(mut slot) = permission_panel_geometry_slot().lock() {
        *slot = None;
    }
    #[cfg(test)]
    {}
}

/// Draw the in-chat permission panel as a floating card anchored directly
/// above the composer (falling back to lower-center of `area` when no
/// composer snapshot exists yet). Pills stack line-by-line; geometry is
/// recorded for mouse hit-testing.
pub(super) fn draw_permission_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &crate::tui::PermissionPanelState,
) {
    use crate::tui::PermissionPanelDecision;

    let dim_style = Style::default().fg(dim_color());
    let value_style = Style::default().fg(user_text());

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Plan agent wants to ", value_style),
        Span::styled(panel.request.tool_name.to_uppercase(), Style::default().fg(user_color()).bold()),
    ]));
    if !panel.request.path.is_empty() {
        lines.push(Line::from(Span::styled(
            panel.request.path.clone(),
            value_style,
        )));
    }
    lines.push(Line::from(Span::styled(
        format!("Reason: {}", panel.request.reason),
        dim_style,
    )));
    lines.push(Line::from(""));

    // Stacked pills, one per line. The selected row gets the ❯ marker and
    // the Kraivcode accent treatment.
    for (index, decision) in PermissionPanelDecision::PILLS.into_iter().enumerate() {
        let selected = index == panel.selected;
        let marker = if selected { "❯ " } else { "  " };
        let style = if selected {
            Style::default()
                .fg(user_color())
                .bg(user_bg())
                .add_modifier(Modifier::BOLD)
        } else {
            dim_style
        };
        lines.push(Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("◖ {} ◗", decision.label()), style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "↑/↓ select · Enter confirm · Esc dismiss",
        dim_style,
    )));

    let content_width = lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(0)
        .clamp(34, area.width.saturating_sub(4) as usize);
    let boxed = render_rounded_box(
        "Permission required",
        lines,
        content_width + 6,
        Style::default().fg(user_color()),
    );

    let box_width = boxed.iter().map(|line| line.width()).max().unwrap_or(0) as u16;
    let box_height = boxed.len() as u16;
    if box_width == 0 || box_width > area.width || box_height > area.height {
        return;
    }

    // Anchor: bottom of the card sits just above the composer's top border.
    let anchor_y = crate::tui::ui::composer_anchor_rect()
        .map(|rect| rect.y.saturating_sub(box_height + 1))
        .unwrap_or_else(|| area.y + area.height.saturating_sub(box_height + 2));
    let anchor_y = anchor_y.max(area.y);
    let card = Rect {
        x: area.x + (area.width.saturating_sub(box_width)) / 2,
        y: anchor_y,
        width: box_width,
        height: box_height,
    };

    // Record pill rects: pills stack one per line, so each line containing a
    // ◖ maps to the next decision in order.
    let mut pills: Vec<(Rect, PermissionPanelDecision)> = Vec::new();
    for (index, line) in boxed.iter().enumerate() {
        let plain: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        let Some(marker_offset) = plain.find("◖") else {
            continue;
        };
        let Some(decision) = PermissionPanelDecision::PILLS.get(pills.len()).copied() else {
            break;
        };
        let label_end = plain[marker_offset..]
            .find("◗")
            .map(|position| position + 2)
            .unwrap_or(plain.len() - marker_offset);
        pills.push((
            Rect {
                x: card.x + marker_offset as u16,
                y: card.y + 1 + index as u16,
                width: label_end as u16,
                height: 1,
            },
            decision,
        ));
    }

    store_permission_panel_geometry(PermissionPanelGeometry {
        card,
        pills,
    });

    frame.render_widget(ratatui::widgets::Clear, card);
    for (index, line) in boxed.iter().enumerate() {
        let row = Rect {
            x: card.x,
            y: card.y + index as u16,
            width: card.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(line.clone()), row);
    }
}

/// Draw the right-click "Model details" card centered over `area`.
///
/// Renders key/value facts, the two action pills, and records their screen
/// rects for mouse hit-testing. Called late in the frame so the card floats
/// above the picker, transcript, and info widgets.
pub(super) fn draw_model_detail_popup(frame: &mut Frame, area: Rect, popup: &crate::tui::ModelDetailPopup) {
    use crate::tui::ModelDetailButton;

    let dim_value_style = Style::default().fg(dim_color());
    let value_style = Style::default().fg(user_text());
    let kv = |key: &str, value: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("{key:<11}"), dim_value_style),
            Span::styled(value.to_string(), value_style),
        ])
    };

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(kv("Model", popup.model_name.as_str()));
    lines.push(kv("Spec", popup.model_spec.as_str()));
    lines.push(kv("Provider", popup.provider_label.as_str()));
    lines.push(kv("Login", popup.login_method.as_str()));
    lines.push(kv("API", popup.api_method.as_str()));
    lines.push(kv(
        "Base URL",
        popup.base_url.as_deref().unwrap_or("—"),
    ));
    let default_suffix = if popup.is_default { "  ◆ current default" } else { "" };
    lines.push(kv(
        "Status",
        format!("{}{}", popup.status, default_suffix).as_str(),
    ));

    // Action pills.
    let pill = |button: ModelDetailButton, selected: bool| -> (Span<'static>, usize) {
        let label = button.label();
        let text = format!("◖ {label} ◗");
        let style = if selected {
            Style::default()
                .fg(user_color())
                .bg(user_bg())
                .add_modifier(Modifier::BOLD)
        } else {
            dim_value_style
        };
        (Span::styled(text, style), label.chars().count() + 4)
    };
    let (set_span, _) = pill(
        ModelDetailButton::SetDefault,
        popup.selected_button == ModelDetailButton::SetDefault,
    );
    let (sel_span, _) = pill(
        ModelDetailButton::SelectSession,
        popup.selected_button == ModelDetailButton::SelectSession,
    );
    let buttons_line = Line::from(vec![
        Span::raw(" "),
        set_span,
        Span::raw("   "),
        sel_span,
        Span::raw(" "),
    ]);
    lines.push(Line::from(""));
    lines.push(buttons_line);

    let content_width = lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(0)
        .clamp(30, area.width.saturating_sub(4) as usize);
    let border_style = Style::default().fg(user_color());
    let boxed = render_rounded_box("Model details", lines, content_width + 6, border_style);

    let box_width = boxed.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let box_height = boxed.len() as u16;
    if box_width == 0 || box_width > area.width || box_height + 2 > area.height {
        return;
    }
    let card = Rect {
        x: area.x + (area.width.saturating_sub(box_width)) / 2,
        y: area.y + (area.height.saturating_sub(box_height)) / 2,
        width: box_width,
        height: box_height,
    };

    // Locate each pill's x range inside the composed plain text so click
    // rects match what is on screen exactly.
    let plain_for = |index: usize| -> String {
        boxed
            .get(index)
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .unwrap_or_default()
    };
    let button_line_index = boxed
        .iter()
        .position(|line| line.spans.iter().any(|span| span.content.contains('◖')))
        .unwrap_or(0);
    let button_plain = plain_for(button_line_index);
    let mut button_rects: Vec<(Rect, ModelDetailButton)> = Vec::new();
    let mut search_from = 0usize;
    for (button, label) in [
        (ModelDetailButton::SetDefault, ModelDetailButton::SetDefault.label()),
        (
            ModelDetailButton::SelectSession,
            ModelDetailButton::SelectSession.label(),
        ),
    ] {
        let needle_start = format!("◖ {label}");
        if let Some(relative) = button_plain[search_from..].find(&needle_start) {
            let start = search_from + relative;
            let rect = Rect {
                x: card.x + start as u16,
                y: card.y + 1 + button_line_index as u16,
                width: needle_start.chars().count() as u16 + 2, // + trailing ◗ and space
                height: 1,
            };
            button_rects.push((rect, button));
            search_from = start + needle_start.len();
        }
    }

    store_model_detail_popup_geometry(ModelDetailPopupGeometry {
        card,
        buttons: button_rects,
    });

    frame.render_widget(ratatui::widgets::Clear, card);
    for (index, line) in boxed.iter().enumerate() {
        let row = Rect {
            x: card.x,
            y: card.y + index as u16,
            width: card.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(line.clone()), row);
    }
}

/// Plan-mode `ask_user` popup: question, selectable options, and an optional
/// free-text field. Draws as a centered dark panel matching the skills picker theme.
pub(super) fn draw_ask_user_popup(
    frame: &mut Frame,
    area: Rect,
    popup: &crate::tui::app::agent_persona::PendingAskUser,
) {
    // Center: 60% width, ~50% height
    let panel_w = (area.width * 60 / 100).max(40).min(area.width);
    let panel_h = (area.height * 50 / 100).max(10).min(area.height);
    let panel_x = area.x + (area.width.saturating_sub(panel_w)) / 2;
    let panel_y = area.y + (area.height.saturating_sub(panel_h)) / 2;
    let panel = Rect::new(panel_x, panel_y, panel_w, panel_h);

    // Clear the area, then fill with dark background using space characters.
    // Clear blanks the rect; styled spaces ensure every cell gets PANEL_BG.
    frame.render_widget(ratatui::widgets::Clear, panel);
    let fill_lines: Vec<Line> = (0..panel.height)
        .map(|_| Line::from(Span::styled(" ".repeat(panel.width as usize), Style::default().bg(PANEL_BG))))
        .collect();
    frame.render_widget(Paragraph::new(fill_lines), panel);

    // Outer block: brown border, dark background, hotkey chips in bottom border
    let block = Block::default()
        .title(Span::styled(
            " Agent question ",
            Style::default().fg(Color::White).bold(),
        ))
        .title_bottom(Line::from(vec![
            hotkey(" ↑/↓ "),
            Span::styled(" select  ", Style::default().fg(MUTED_DARK)),
            hotkey(" j/k "),
            Span::styled(" navigate  ", Style::default().fg(MUTED_DARK)),
            hotkey(" Enter "),
            Span::styled(" submit  ", Style::default().fg(MUTED_DARK)),
            hotkey(" Esc "),
            Span::styled(" cancel ", Style::default().fg(MUTED_DARK)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PANEL_BORDER))
        .style(Style::default().bg(PANEL_BG));

    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let inner_width = inner.width as usize;

    // ── Build content lines ──────────────────────────────────────────
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Question label + text
    lines.push(Line::from(vec![
        Span::styled("Question ", Style::default().fg(MUTED_DARK).bold()),
        Span::styled(
            "— the agent asked before continuing:".to_string(),
            Style::default().fg(MUTED),
        ),
    ]));
    lines.push(Line::from(""));
    for wrapped in wrap_plain(&popup.question, inner_width.saturating_sub(2)) {
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(wrapped, Style::default().fg(Color::White)),
        ]));
    }

    // Options
    if !popup.options.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Choose an answer:",
            Style::default().fg(MUTED_DARK).bold(),
        )));
        for (index, (label, _value)) in popup.options.iter().enumerate() {
            let selected = index == popup.selected;
            if selected {
                let mut spans: Vec<Span> = Vec::new();
                spans.push(Span::styled("  ", Style::default().bg(SELECTED_BG)));
                spans.push(Span::styled(
                    "❯ ",
                    Style::default()
                        .fg(ACCENT)
                        .bg(SELECTED_BG)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    label.clone(),
                    Style::default()
                        .fg(Color::White)
                        .bg(SELECTED_BG)
                        .add_modifier(Modifier::BOLD),
                ));
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled("○ ", Style::default().fg(MUTED_DARK)),
                    Span::styled(label.clone(), Style::default().fg(MUTED)),
                ]));
            }
        }
    }

    // Free-text input
    if popup.free_text {
        lines.push(Line::from(""));
        if popup.options.is_empty() {
            lines.push(Line::from(Span::styled(
                " Type your answer:",
                Style::default().fg(MUTED_DARK).bold(),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                " …or type your own answer:",
                Style::default().fg(MUTED_DARK).italic(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(free_text_line(popup, inner_width.saturating_sub(2)));
    }

    // ── Render content into the inner area ───────────────────────────
    // We need to handle overflow: if content is taller than inner, show bottom.
    let total_lines = lines.len();
    let visible_height = inner.height as usize;
    let scroll = if total_lines > visible_height {
        (total_lines - visible_height) as u16
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(PANEL_BG))
        .scroll((scroll, 0));
    frame.render_widget(paragraph, inner);
}

/// Build the free-text field line: a windowed slice of the buffer with the
/// cursor drawn as an inverted block, scrolled so the cursor stays visible.
fn free_text_line(
    popup: &crate::tui::app::agent_persona::PendingAskUser,
    width: usize,
) -> Line<'static> {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    let width = width.max(2);
    let buffer = &popup.free_text_buffer;
    let cursor = popup.cursor.min(buffer.chars().count());

    // Choose the window start so the cursor is always on screen.
    let keep_before = width.saturating_sub(1);
    let start_char = buffer
        .char_indices()
        .enumerate()
        .find_map(|(idx, (char_index, _))| {
            if idx >= cursor.saturating_sub(keep_before) {
                Some(idx)
            } else {
                let _ = char_index;
                None
            }
        })
        .unwrap_or(0);

    // Build the visible window: up to `width` chars starting at start_char.
    let mut visible = String::new();
    for (index, ch) in buffer.chars().enumerate() {
        if index < start_char {
            continue;
        }
        let cw = ch.width().unwrap_or(0);
        if visible.width() + cw > width {
            break;
        }
        visible.push(ch);
    }

    // Split `visible` at the cursor (relative to the window start).
    let cursor_rel = cursor.saturating_sub(start_char);
    let mut split = visible.len();
    for (idx, (byte_index, _)) in visible.char_indices().enumerate() {
        if idx >= cursor_rel {
            split = byte_index;
            break;
        }
    }

    Line::from(vec![
        Span::styled("❯ ", Style::default().fg(ACCENT)),
        Span::styled(
            visible[..split].to_string(),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            "▊",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            visible[split..].to_string(),
            Style::default().fg(MUTED),
        ),
    ])
}

/// Simple word-wrap of plain text to a target width, splitting on whitespace.
fn wrap_plain(text: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let width = width.max(4);
    let mut out: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut line_width = 0usize;
        for word in paragraph.split_whitespace() {
            let word_width = word.width();
            let sep = if line.is_empty() { 0 } else { 1 };
            if line_width + sep + word_width > width && !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line_width = 0;
            }
            if !line.is_empty() {
                line.push(' ');
                line_width += 1;
            }
            line.push_str(word);
            line_width += word_width;
        }
        if !line.is_empty() {
            out.push(line);
        } else if paragraph.is_empty() && out.is_empty() {
            out.push(String::new());
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
