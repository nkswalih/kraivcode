//! `/skills` dialog panel.
//!
//! A searchable overlay that replaces the old plain-text `/skills` report. It
//! shows loaded skills (activable with Enter) alongside jcode-endorsed skills
//! that are not yet installed (Enter copies the install command / source to the
//! clipboard). The plain-text report stays reachable through the hidden
//! `/skills-text` fallback command.
//!
//! The panel mirrors the account picker (`jcode-tui-account-picker`) UX: a
//! centered box with a filter row up top, a list/detail split in the middle,
//! category-group navigation with Left/Right, and the hotkey row drawn into the
//! bottom border.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

const PANEL_BG: Color = Color::Rgb(16, 16, 14);
const PANEL_BORDER: Color = Color::Rgb(84, 64, 40);
const PANEL_BORDER_ACTIVE: Color = Color::Rgb(255, 140, 0);
const SECTION_BORDER: Color = Color::Rgb(110, 88, 62);
const SELECTED_BG: Color = Color::Rgb(46, 32, 16);
const MUTED: Color = Color::Rgb(172, 152, 112);
const MUTED_DARK: Color = Color::Rgb(118, 100, 74);
const ACCENT: Color = Color::Rgb(255, 140, 0);
const INSTALLED: Color = Color::Rgb(110, 214, 158);
const HINT: Color = Color::Rgb(170, 210, 255);
const GOLD: Color = Color::Rgb(229, 187, 111);
const OVERLAY_PERCENT_X: u16 = 88;
const OVERLAY_PERCENT_Y: u16 = 74;

/// One row in the skills panel.
#[derive(Debug, Clone)]
pub struct SkillItem {
    /// Slash-command name (`name` in SKILL.md / the endorsed catalog).
    pub name: String,
    /// Grouping label. Loaded skills use the `"Loaded"` group; endorsed skills
    /// use their catalog category (e.g. `"NVIDIA CUDA-X"`).
    pub category: String,
    pub description: String,
    /// Path (loaded) or source/catalog note (endorsed).
    pub source: String,
    /// Optional install command for a not-yet-installed endorsed skill.
    pub install: Option<String>,
    /// True when the skill is loaded (so Enter activates it).
    pub installed: bool,
    /// True when this is the currently active skill.
    pub active: bool,
}

/// What happens when the user presses Enter on a selected row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillPickerCommand {
    /// Activate a loaded skill in the foreground session.
    Activate { name: String },
    /// Copy the install command (or source) to the clipboard.
    Copy { text: String },
}

#[derive(Debug)]
pub enum OverlayAction {
    Continue,
    Close,
    Execute(SkillPickerCommand),
}

#[derive(Debug)]
pub struct SkillPicker {
    title: String,
    items: Vec<SkillItem>,
    filtered: Vec<usize>,
    selected: usize,
    filter: String,
    last_list_area: Option<Rect>,
}

impl SkillPicker {
    pub fn new(title: impl Into<String>, items: Vec<SkillItem>) -> Self {
        let mut picker = Self {
            title: title.into(),
            items,
            filtered: Vec::new(),
            selected: 0,
            filter: String::new(),
            last_list_area: None,
        };
        picker.apply_filter();
        picker
    }

    fn selected_item(&self) -> Option<&SkillItem> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.items.get(*idx))
    }

    fn visible_window_start(&self, available_items: usize) -> usize {
        self.selected
            .saturating_sub(available_items.saturating_sub(1).min(available_items / 2))
    }

    fn visible_index_for_row(&self, row: u16, list_height: u16) -> Option<usize> {
        if self.filtered.is_empty() {
            return None;
        }

        let available_items = (list_height as usize).max(1);
        let start = self.visible_window_start(available_items);
        let end = (start + available_items).min(self.filtered.len());
        let mut current_category: Option<&str> = None;
        let mut rendered_row = 0u16;

        for visible_idx in start..end {
            let item = &self.items[self.filtered[visible_idx]];
            if current_category != Some(item.category.as_str()) {
                current_category = Some(item.category.as_str());
                if rendered_row == row {
                    return None;
                }
                rendered_row = rendered_row.saturating_add(1);
                if rendered_row >= list_height {
                    return None;
                }
            }

            if rendered_row == row {
                return Some(visible_idx);
            }
            rendered_row = rendered_row.saturating_add(1);
            if rendered_row > row && rendered_row >= list_height {
                return None;
            }
        }

        None
    }

    fn apply_filter(&mut self) {
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| item_matches_filter(item, &self.filter).then_some(idx))
            .collect();
        self.filtered.sort_by(|left, right| {
            let left_item = &self.items[*left];
            let right_item = &self.items[*right];
            category_rank(&left_item.category)
                .cmp(&category_rank(&right_item.category))
                .then_with(|| left_item.name.cmp(&right_item.name))
                .then_with(|| left.cmp(right))
        });
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn select_prev_category_group(&mut self) {
        let Some(current_idx) = self.filtered.get(self.selected).copied() else {
            return;
        };
        let current_category = self.items[current_idx].category.as_str();
        let mut target = None;

        for pos in (0..self.selected).rev() {
            let category = self.items[self.filtered[pos]].category.as_str();
            if category != current_category {
                target = Some(pos);
                break;
            }
        }

        let Some(mut pos) = target else {
            return;
        };
        let category = self.items[self.filtered[pos]].category.clone();
        while pos > 0 && self.items[self.filtered[pos - 1]].category == category {
            pos -= 1;
        }
        self.selected = pos;
    }

    fn select_next_category_group(&mut self) {
        let Some(current_idx) = self.filtered.get(self.selected).copied() else {
            return;
        };
        let current_category = self.items[current_idx].category.as_str();

        for pos in (self.selected + 1)..self.filtered.len() {
            let category = self.items[self.filtered[pos]].category.as_str();
            if category != current_category {
                self.selected = pos;
                break;
            }
        }
    }

    fn filtered_category_count(&self, category: &str) -> usize {
        self.filtered
            .iter()
            .filter(|idx| self.items[**idx].category == category)
            .count()
    }

    pub fn handle_overlay_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<OverlayAction> {
        match code {
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.apply_filter();
                    return Ok(OverlayAction::Continue);
                }
                return Ok(OverlayAction::Close);
            }
            KeyCode::Char('q') if !modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(OverlayAction::Close);
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(OverlayAction::Close);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.filtered.len().saturating_sub(1);
                self.selected = (self.selected + 1).min(max);
            }
            KeyCode::Left => {
                self.select_prev_category_group();
            }
            KeyCode::Right => {
                self.select_next_category_group();
            }
            KeyCode::PageUp | KeyCode::Char('K') => {
                self.selected = self.selected.saturating_sub(6);
            }
            KeyCode::PageDown | KeyCode::Char('J') => {
                let max = self.filtered.len().saturating_sub(1);
                self.selected = (self.selected + 6).min(max);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = self.filtered.len().saturating_sub(1);
            }
            KeyCode::Backspace => {
                if self.filter.pop().is_some() {
                    self.apply_filter();
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.selected_item() {
                    if item.installed {
                        return Ok(OverlayAction::Execute(SkillPickerCommand::Activate {
                            name: item.name.clone(),
                        }));
                    }
                    let copy_text = item.install.clone().unwrap_or_else(|| item.source.clone());
                    return Ok(OverlayAction::Execute(SkillPickerCommand::Copy {
                        text: copy_text,
                    }));
                }
                return Ok(OverlayAction::Close);
            }
            KeyCode::Char(c)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT) =>
            {
                self.filter.push(c);
                self.apply_filter();
            }
            _ => {}
        }
        Ok(OverlayAction::Continue)
    }

    pub fn handle_overlay_mouse(&mut self, mouse: MouseEvent) {
        let Some(list_inner) = self.last_list_area else {
            return;
        };
        let inside_list = mouse.column >= list_inner.x
            && mouse.column < list_inner.x.saturating_add(list_inner.width)
            && mouse.row >= list_inner.y
            && mouse.row < list_inner.y.saturating_add(list_inner.height);

        match mouse.kind {
            MouseEventKind::ScrollUp if inside_list => {
                self.selected = self.selected.saturating_sub(1);
            }
            MouseEventKind::ScrollDown if inside_list => {
                let max = self.filtered.len().saturating_sub(1);
                self.selected = (self.selected + 1).min(max);
            }
            MouseEventKind::Down(MouseButton::Left) if inside_list => {
                let row = mouse.row.saturating_sub(list_inner.y);
                if let Some(visible_idx) = self.visible_index_for_row(row, list_inner.height) {
                    self.selected = visible_idx;
                }
            }
            _ => {}
        }
    }

    /// Estimated in-memory footprint for the `/debug` memory profile.
    pub fn debug_memory_profile(&self) -> serde_json::Value {
        let items_bytes: usize = self
            .items
            .iter()
            .map(|item| {
                item.name.capacity()
                    + item.category.capacity()
                    + item.description.capacity()
                    + item.source.capacity()
                    + item.install.as_ref().map(|s| s.capacity()).unwrap_or(0)
            })
            .sum();
        let filtered_bytes = self.filtered.capacity() * std::mem::size_of::<usize>();
        let total = items_bytes + filtered_bytes + self.filter.capacity() + self.title.capacity();

        serde_json::json!({
            "items_count": self.items.len(),
            "filtered_count": self.filtered.len(),
            "selected": self.selected,
            "total_estimate_bytes": total,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_rect(OVERLAY_PERCENT_X, OVERLAY_PERCENT_Y, frame.area());

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .title_bottom(Line::from(vec![
                hotkey(" Enter "),
                Span::styled(" activate / copy  ", Style::default().fg(MUTED_DARK)),
                hotkey(" Left/Right "),
                Span::styled(" jump group  ", Style::default().fg(MUTED_DARK)),
                hotkey(" Click "),
                Span::styled(" select  ", Style::default().fg(MUTED_DARK)),
                hotkey(" type "),
                Span::styled(" filter  ", Style::default().fg(MUTED_DARK)),
                hotkey(" Esc "),
                Span::styled(" clear / close ", Style::default().fg(MUTED_DARK)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PANEL_BORDER));
        frame.render_widget(block, area);

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(inner);

        self.render_header(frame, rows[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(rows[1]);

        self.render_list(frame, body[0]);
        self.render_detail_pane(frame, body[1]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Hint ", Style::default().fg(MUTED_DARK)),
            Span::styled(
                "Enter activates a loaded skill, or copies the install command for a recommended one. Type to filter; Left/Right jump between groups.",
                Style::default().fg(MUTED),
            ),
        ]));
        frame.render_widget(footer, rows[2]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(
                " Overview ",
                Style::default().fg(Color::White).bold(),
            ))
            .borders(Borders::ALL)
            .style(Style::default().bg(PANEL_BG))
            .border_style(Style::default().fg(SECTION_BORDER));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let active = self
            .items
            .iter()
            .find(|item| item.active)
            .map(|item| item.name.clone());

        let loaded = self.items.iter().filter(|item| item.installed).count();
        let to_install = self.items.len().saturating_sub(loaded);

        let active_span = match &active {
            Some(name) => Span::styled(name.clone(), Style::default().fg(INSTALLED).bold()),
            None => Span::styled("none", Style::default().fg(MUTED)),
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Filter ", Style::default().fg(MUTED_DARK)),
                Span::styled(
                    if self.filter.is_empty() {
                        "type a skill name or topic".to_string()
                    } else {
                        self.filter.clone()
                    },
                    if self.filter.is_empty() {
                        Style::default().fg(Color::Gray).italic()
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::styled(
                    format!(
                        "  -  {} result{}",
                        self.filtered.len(),
                        if self.filtered.len() == 1 { "" } else { "s" }
                    ),
                    Style::default().fg(MUTED_DARK),
                ),
            ]),
            Line::from(vec![
                metric_span("loaded", loaded, INSTALLED),
                Span::raw("  "),
                metric_span("to install", to_install, ACCENT),
                Span::raw("  "),
                Span::styled("active ", Style::default().fg(MUTED_DARK)),
                active_span,
            ]),
            Line::from(vec![
                Span::styled("Usage ", Style::default().fg(MUTED_DARK)),
                Span::styled(
                    "Enter activates a loaded skill or copies the install command for a recommended one.",
                    Style::default().fg(MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled("Tip ", Style::default().fg(MUTED_DARK)),
                Span::styled(
                    "/skills-text shows the full plain-text report.",
                    Style::default().fg(MUTED),
                ),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        let title = if self.filtered.is_empty() {
            " Skills ".to_string()
        } else {
            format!(" Skills ({}/{}) ", self.selected + 1, self.filtered.len())
        };
        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default().fg(Color::White).bold(),
            ))
            .borders(Borders::ALL)
            .style(Style::default().bg(PANEL_BG))
            .border_style(Style::default().fg(PANEL_BORDER_ACTIVE));
        let list_inner = block.inner(area);
        frame.render_widget(block, area);
        self.last_list_area = Some(list_inner);

        let available_items = (list_inner.height as usize).max(1);
        let start = self.visible_window_start(available_items);
        let end = (start + available_items).min(self.filtered.len());

        let mut lines = Vec::new();
        if self.filtered.is_empty() {
            lines.push(Line::from(Span::styled(
                "No skills match your filter.",
                Style::default().fg(Color::Gray).italic(),
            )));
            lines.push(Line::from(Span::styled(
                "Try a skill name, or clear the filter with Esc.",
                Style::default().fg(MUTED),
            )));
        } else {
            let mut current_category: Option<String> = None;
            for visible_idx in start..end {
                let idx = self.filtered[visible_idx];
                let item = &self.items[idx];
                let selected = visible_idx == self.selected;

                if current_category.as_deref() != Some(item.category.as_str()) {
                    current_category = Some(item.category.clone());
                    lines.push(category_header_line(
                        &item.category,
                        self.filtered_category_count(&item.category),
                    ));
                }

                let row_style = if selected {
                    Style::default().bg(SELECTED_BG)
                } else {
                    Style::default()
                };
                let (glyph, glyph_color) = item_glyph(item);
                let mut row = vec![
                    Span::styled(
                        if selected { "> " } else { "  " },
                        row_style.fg(Color::White),
                    ),
                    Span::styled(format!("{} ", glyph), row_style.fg(glyph_color).bold()),
                    Span::styled(
                        truncate_with_ellipsis(&item.name, 26),
                        row_style.fg(Color::White),
                    ),
                ];
                if item.active {
                    row.push(Span::styled("  [active]", row_style.fg(INSTALLED).bold()));
                }
                if !item.installed {
                    row.push(Span::styled("  [install]", row_style.fg(ACCENT)));
                }
                lines.push(Line::from(row));
            }
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), list_inner);
    }

    fn render_detail_pane(&self, frame: &mut Frame, area: Rect) {
        let title = self
            .selected_item()
            .map(|item| format!(" {} ", item.name))
            .unwrap_or_else(|| " Details ".to_string());
        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default().fg(Color::White).bold(),
            ))
            .borders(Borders::ALL)
            .style(Style::default().bg(PANEL_BG))
            .border_style(Style::default().fg(SECTION_BORDER));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(item) = self.selected_item() else {
            frame.render_widget(
                Paragraph::new("No skill selected").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return;
        };

        let (glyph, glyph_color) = item_glyph(item);
        let status_label = if item.active {
            "active"
        } else if item.installed {
            "loaded"
        } else {
            "not installed"
        };
        let status_color = if item.active || item.installed {
            INSTALLED
        } else {
            ACCENT
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", glyph),
                    Style::default().fg(glyph_color).bold(),
                ),
                Span::styled(status_label, Style::default().fg(status_color).bold()),
                Span::styled("  -  ", Style::default().fg(MUTED_DARK)),
                Span::styled(item.category.clone(), Style::default().fg(GOLD)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                truncate_with_ellipsis(&item.description, inner.width.saturating_sub(2) as usize),
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Source ",
                Style::default().fg(MUTED_DARK).bold(),
            )]),
            Line::from(vec![Span::styled(
                truncate_with_ellipsis(&item.source, inner.width.saturating_sub(2) as usize),
                Style::default().fg(MUTED),
            )]),
            Line::from(""),
        ];

        if let Some(install) = &item.install
            && !item.installed
        {
            lines.push(Line::from(vec![Span::styled(
                "Install",
                Style::default().fg(MUTED_DARK).bold(),
            )]));
            lines.push(Line::from(vec![Span::styled(
                truncate_with_ellipsis(install, inner.width.saturating_sub(2) as usize),
                Style::default().fg(Color::White),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Press Enter to copy the install command to your clipboard.",
                Style::default().fg(HINT),
            )]));
        } else if !item.installed {
            lines.push(Line::from(vec![Span::styled(
                "Install manually from the source above, then run /skills or skill_manage reload_all.",
                Style::default().fg(MUTED),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "Press Enter to copy the source to your clipboard.",
                Style::default().fg(HINT),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                truncate_with_ellipsis(
                    &format!("Loaded from {}", item.source,),
                    inner.width.saturating_sub(2) as usize,
                ),
                Style::default().fg(MUTED),
            )]));
            lines.push(Line::from(""));
            if item.active {
                lines.push(Line::from(vec![Span::styled(
                    "This skill is the currently active skill.",
                    Style::default().fg(INSTALLED).bold(),
                )]));
                lines.push(Line::from(vec![Span::styled(
                    "Press Enter to keep it active.",
                    Style::default().fg(MUTED),
                )]));
            } else {
                lines.push(Line::from(vec![Span::styled(
                    "Press Enter to activate this skill.",
                    Style::default().fg(HINT),
                )]));
            }
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}

fn category_rank(category: &str) -> usize {
    if category == "Loaded" {
        0
    } else {
        1 + category_version_space(category)
    }
}

/// Stable secondary ordering for endorsed catalog groups so they follow the
/// first-seen order of `endorsed_skills()` rather than an arbitrary hash.
fn category_version_space(category: &str) -> usize {
    crate::skill::endorsed_skills()
        .iter()
        .position(|endorsed| endorsed.category == category)
        .unwrap_or(usize::MAX / 2)
}

fn item_matches_filter(item: &SkillItem, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {}",
        item.name.to_lowercase(),
        item.description.to_lowercase(),
        item.category.to_lowercase(),
        item.source.to_lowercase()
    );
    haystack.contains(&filter.to_lowercase())
}

fn item_glyph(item: &SkillItem) -> (&'static str, Color) {
    if item.active {
        ("*", INSTALLED)
    } else if item.installed {
        ("+", GOLD)
    } else {
        ("!", ACCENT)
    }
}

fn category_header_line(category: &str, count: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            category.to_string(),
            Style::default()
                .fg(if category == "Loaded" {
                    INSTALLED
                } else {
                    GOLD
                })
                .bold(),
        ),
        Span::styled(
            format!("  -  {} skill{}", count, if count == 1 { "" } else { "s" }),
            Style::default().fg(MUTED_DARK),
        ),
    ])
}

fn metric_span(label: &'static str, value: usize, color: Color) -> Span<'static> {
    Span::styled(
        format!("{} {}", label, value),
        Style::default().fg(color).bold(),
    )
}

fn hotkey(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::default().fg(Color::White).bg(Color::DarkGray))
}

fn truncate_with_ellipsis(input: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= width {
        return input.to_string();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let mut out: String = chars.into_iter().take(width - 3).collect();
    out.push_str("...");
    out
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
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
        .split(popup[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

    fn sample_items() -> Vec<SkillItem> {
        vec![
            SkillItem {
                name: "optimization".to_string(),
                category: "Loaded".to_string(),
                description: "Improve performance by measuring and attributing bottlenecks."
                    .to_string(),
                source: ".jcode/skills/optimization".to_string(),
                install: None,
                installed: true,
                active: true,
            },
            SkillItem {
                name: "firefox-browser".to_string(),
                category: "Loaded".to_string(),
                description: "Control the user's Firefox browser.".to_string(),
                source: "~/.jcode/skills/firefox-browser".to_string(),
                install: None,
                installed: true,
                active: false,
            },
            SkillItem {
                name: "frontend-design".to_string(),
                category: "Anthropic Design".to_string(),
                description: "Create distinctive, production-grade frontend interfaces."
                    .to_string(),
                source: "anthropics/skills (official Anthropic catalog)".to_string(),
                install: Some(
                    "npx skills add anthropics/skills --skill frontend-design --yes".to_string(),
                ),
                installed: false,
                active: false,
            },
        ]
    }

    fn buffer_to_text(buffer: &ratatui::buffer::Buffer) -> String {
        let area = buffer.area;
        let mut out = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                out.push_str(buffer[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn text_contains_wrapped(rendered: &str, expected: &str) -> bool {
        if rendered.contains(expected) {
            return true;
        }
        let tokens = expected.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            return true;
        }
        let mut start = 0;
        for token in tokens {
            let Some(offset) = rendered[start..].find(token) else {
                return false;
            };
            start += offset + token.len();
        }
        true
    }

    #[test]
    fn renders_items_filter_and_recommended_markers() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());

        let backend = TestBackend::new(140, 40);
        let mut terminal = Terminal::new(backend).expect("failed to create terminal");
        terminal
            .draw(|frame| picker.render(frame))
            .expect("draw failed");
        let text = buffer_to_text(terminal.backend().buffer());

        for expected in [
            "optimization",
            "firefox-browser",
            "frontend-design",
            "Loaded",
            "Anthropic Design",
            "[active]",
            "[install]",
            "3 results",
        ] {
            assert!(
                text_contains_wrapped(&text, expected),
                "skill picker missing {expected:?}: {text}"
            );
        }
    }

    #[test]
    fn filter_narrows_by_name_and_keeps_selection_in_bounds() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        assert_eq!(picker.filtered.len(), 3);

        picker
            .handle_overlay_key(KeyCode::Char('f'), KeyModifiers::empty())
            .expect("typing a filter char is handled");
        assert_eq!(picker.filtered.len(), 2);

        picker
            .handle_overlay_key(KeyCode::Backspace, KeyModifiers::empty())
            .expect("backspace is handled");
        assert_eq!(picker.filtered.len(), 3);
    }

    #[test]
    fn enter_activates_loaded_and_copies_install_for_recommended() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        let first_loaded = picker.items[picker.filtered[0]].name.clone();
        match picker
            .handle_overlay_key(KeyCode::Enter, KeyModifiers::empty())
            .expect("enter should be handled")
        {
            OverlayAction::Execute(SkillPickerCommand::Activate { name }) => {
                assert_eq!(name, first_loaded)
            }
            other => panic!("expected Activate, got {other:?}"),
        }
    }

    #[test]
    fn test_enter_on_recommended_skill_copies_install_command() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        // Jump to the Anthropic Design group (last filtered item).
        picker.selected = picker.filtered.len() - 1;
        match picker
            .handle_overlay_key(KeyCode::Enter, KeyModifiers::empty())
            .expect("enter should be handled")
        {
            OverlayAction::Execute(SkillPickerCommand::Copy { text }) => {
                assert!(text.contains("npx skills add anthropics/skills"), "{text}")
            }
            other => panic!("expected Copy, got {other:?}"),
        }
    }

    #[test]
    fn esc_clears_filter_then_closes() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        picker
            .handle_overlay_key(KeyCode::Char('f'), KeyModifiers::empty())
            .expect("type filter char");
        assert!(matches!(
            picker
                .handle_overlay_key(KeyCode::Esc, KeyModifiers::empty())
                .expect("esc handled"),
            OverlayAction::Continue
        ));
        assert!(picker.filter.is_empty());
        assert!(matches!(
            picker
                .handle_overlay_key(KeyCode::Esc, KeyModifiers::empty())
                .expect("esc handled"),
            OverlayAction::Close
        ));
    }

    #[test]
    fn left_right_jump_between_category_groups() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        // Start in the Loaded group, jump past Anthropic Design and wrap via End.
        picker.selected = 0;
        picker
            .handle_overlay_key(KeyCode::Right, KeyModifiers::empty())
            .expect("right handled");
        assert_eq!(
            picker.items[picker.filtered[picker.selected]].category,
            "Anthropic Design"
        );
        picker
            .handle_overlay_key(KeyCode::Left, KeyModifiers::empty())
            .expect("left handled");
        assert_eq!(
            picker.items[picker.filtered[picker.selected]].category,
            "Loaded"
        );
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn mouse_click_selects_skill_row_after_category_header() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        let backend = TestBackend::new(120, 32);
        let mut terminal = Terminal::new(backend).expect("failed to create terminal");
        terminal
            .draw(|frame| picker.render(frame))
            .expect("draw failed");

        let list_area = picker
            .last_list_area
            .expect("render should record list area");
        let initially_selected = picker.selected;
        picker.handle_overlay_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: list_area.x + 1,
            row: list_area.y,
            modifiers: KeyModifiers::empty(),
        });
        assert_eq!(
            picker.selected, initially_selected,
            "category header rows must not be selectable"
        );

        // Row 0 = "Loaded" header, row 1 = first loaded skill in sorted order
        // ("firefox-browser" < "optimization").
        picker.handle_overlay_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: list_area.x + 1,
            row: list_area.y + 1,
            modifiers: KeyModifiers::empty(),
        });
        assert_eq!(
            picker.selected_item().map(|item| item.name.as_str()),
            Some("firefox-browser")
        );
    }

    #[test]
    fn preserves_underlying_background_outside_panel() {
        let mut picker = SkillPicker::new(" Skills ", sample_items());
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).expect("failed to create terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                let fill = vec![Line::from("X".repeat(area.width as usize)); area.height as usize];
                frame.render_widget(Paragraph::new(fill), area);
                picker.render(frame);
            })
            .expect("draw failed");

        let overlay = centered_rect(
            OVERLAY_PERCENT_X,
            OVERLAY_PERCENT_Y,
            Rect::new(0, 0, 40, 12),
        );
        let probe = &terminal.backend().buffer()[(overlay.x + overlay.width - 3, overlay.y + 2)];
        assert_eq!(probe.symbol(), "X");
        assert_ne!(probe.bg, PANEL_BG);
    }
}
