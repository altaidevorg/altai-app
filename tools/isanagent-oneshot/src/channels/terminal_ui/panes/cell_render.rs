//! Cell → line rendering shared by transcript and tool history.

use ratatui::prelude::*;
use ratatui::style::Modifier;

use crate::channels::terminal_ui::markdown;
use crate::channels::terminal_ui::{Cell, Theme, ToolNoticePhase};

/// Greedy wrap by display width (`unicode_width`); preserves explicit newlines.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width < 4 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut col = 0usize;
        for ch in paragraph.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch)
                .unwrap_or(0)
                .max(1);
            if col + w > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                col = 0;
            }
            if col == 0 && ch.is_whitespace() {
                continue;
            }
            line.push(ch);
            col += w;
        }
        if !line.is_empty() || paragraph.is_empty() {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(crate) fn cell_block_lines(cell: &Cell, inner_width: usize) -> Vec<Line<'static>> {
    let w = inner_width.max(8);
    match cell {
        Cell::User { text } => {
            let mut v = vec![Line::from(vec![
                Span::styled(" ❯ ", Theme::user_prefix()),
                Span::styled("YOU", Theme::user_prefix()),
            ])];
            for ln in wrap_text(text, w.saturating_sub(3)) {
                v.push(Line::from(vec![
                    Span::styled("   ", Theme::dim()),
                    Span::styled(ln, Theme::text()),
                ]));
            }
            v.push(Line::from(""));
            v
        }
        Cell::Assistant { markdown } => {
            let mut v = vec![Line::from(vec![
                Span::styled(
                    format!(" {} ", crate::channels::terminal_ui::COMPACT_MARK),
                    Theme::active(),
                ),
                Span::styled("ALTAI", Theme::text().add_modifier(Modifier::BOLD)),
            ])];
            v.extend(markdown::assistant_markdown_lines(
                markdown,
                w.saturating_sub(2),
            ));
            v.push(Line::from(""));
            v
        }
        Cell::Thinking { text } => {
            let mut v = vec![Line::from(vec![
                Span::styled(" ◆ ", Theme::active()),
                Span::styled("THINKING", Theme::thinking().add_modifier(Modifier::BOLD)),
            ])];
            for ln in wrap_text(text, w.saturating_sub(3)) {
                v.push(Line::from(vec![
                    Span::styled("   ", Theme::dim()),
                    Span::styled(ln, Theme::thinking()),
                ]));
            }
            v.push(Line::from(""));
            v
        }
        Cell::ToolNotice {
            phase,
            content,
            tool_call_id: _,
        } => {
            let label_style = match phase {
                ToolNoticePhase::Pending => Theme::tool_pending().add_modifier(Modifier::BOLD),
                ToolNoticePhase::Call => Theme::tool_call().add_modifier(Modifier::BOLD),
                ToolNoticePhase::Result => Theme::tool_done().add_modifier(Modifier::BOLD),
                ToolNoticePhase::Failed => Theme::error().add_modifier(Modifier::BOLD),
                ToolNoticePhase::Other => Theme::tool_call().add_modifier(Modifier::BOLD),
            };
            let (icon, label) = match phase {
                ToolNoticePhase::Pending => ("●", "RUN"),
                ToolNoticePhase::Call => ("›", "TOOL"),
                ToolNoticePhase::Result => ("✓", "DONE"),
                ToolNoticePhase::Failed => ("✕", "FAIL"),
                ToolNoticePhase::Other => ("·", "TOOL"),
            };
            let body_style = match phase {
                ToolNoticePhase::Pending => Theme::tool_pending(),
                ToolNoticePhase::Failed => Theme::error(),
                _ => Theme::dim(),
            };
            let label_width = label.len() + 5;
            let wrapped = wrap_text(content, w.saturating_sub(label_width).max(8));
            let mut v = Vec::with_capacity(wrapped.len() + 1);
            for (index, ln) in wrapped.into_iter().enumerate() {
                if index == 0 {
                    v.push(Line::from(vec![
                        Span::styled(format!(" {icon} "), label_style),
                        Span::styled(format!("{label:<5}"), label_style),
                        Span::styled(ln, body_style),
                    ]));
                } else {
                    v.push(Line::from(vec![
                        Span::styled("         ", Theme::dim()),
                        Span::styled(ln, body_style),
                    ]));
                }
            }
            v
        }
        Cell::Clarification {
            text,
            choices,
            edit_diff,
        } => {
            let inner = w.saturating_sub(2).max(8);
            let title = if edit_diff.is_some() {
                "edit approval"
            } else {
                "approval"
            };
            let mut v = vec![Line::from(vec![
                Span::styled(" ! ", Theme::tool_call()),
                Span::styled(
                    title.to_ascii_uppercase(),
                    Theme::clarification().add_modifier(Modifier::BOLD),
                ),
            ])];
            if let Some(diff) = edit_diff {
                v.push(Line::from(vec![
                    Span::styled(" file ", Theme::dim()),
                    Span::styled(diff.file.clone(), Theme::active()),
                ]));
                if diff.truncated {
                    v.push(Line::from(Span::styled(" [truncated]", Theme::tool_call())));
                }
                v.extend(crate::channels::terminal_ui::approval::diff_lines_to_spans(
                    &diff.diff, 40,
                ));
            }
            for ln in wrap_text(text, inner) {
                v.push(Line::from(Span::styled(ln, Theme::clarification())));
            }
            let shown_choices = if choices.is_empty() {
                crate::channels::terminal_ui::APPROVAL_CHOICES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect::<Vec<_>>()
            } else {
                choices.clone()
            };
            if !shown_choices.is_empty() {
                v.push(Line::from(Span::styled(
                    "1 approve · 2 deny · 3 always · 4 abort  (or type the option)",
                    Theme::dim(),
                )));
                let indent = "   ";
                for (i, choice) in shown_choices.iter().enumerate() {
                    let n = i + 1;
                    let head = format!("{n}. ");
                    let first = format!("{head}{choice}");
                    let lines = wrap_text(&first, inner);
                    for (li, seg) in lines.iter().enumerate() {
                        let line = if li == 0 {
                            seg.clone()
                        } else {
                            format!("{indent}{seg}")
                        };
                        v.push(Line::from(Span::styled(line, Theme::clarification())));
                    }
                }
            }
            v.push(Line::from(""));
            v
        }
        Cell::System { message } => {
            let mut v = vec![Line::from(vec![
                Span::styled(" · ", Theme::dim()),
                Span::styled("INFO", Theme::dim().add_modifier(Modifier::BOLD)),
            ])];
            for ln in wrap_text(message, w.saturating_sub(3)) {
                v.push(Line::from(vec![
                    Span::styled("   ", Theme::dim()),
                    Span::styled(ln, Theme::dim()),
                ]));
            }
            v.push(Line::from(""));
            v
        }
        Cell::Error { message } => {
            let mut v = vec![Line::from(vec![
                Span::styled(" ✕ ", Theme::error()),
                Span::styled("ERROR", Theme::error().add_modifier(Modifier::BOLD)),
            ])];
            for ln in wrap_text(message, w.saturating_sub(3)) {
                v.push(Line::from(vec![
                    Span::styled("   ", Theme::dim()),
                    Span::styled(ln, Theme::error()),
                ]));
            }
            v.push(Line::from(""));
            v
        }
    }
}

pub(crate) fn flatten_cells_to_lines(cells: &[Cell], inner_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for cell in cells {
        lines.extend(cell_block_lines(cell, inner_width));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn conversation_roles_use_altai_labels() {
        let cells = vec![
            Cell::User {
                text: "hello".into(),
            },
            Cell::Assistant {
                markdown: "world".into(),
            },
        ];
        let output = plain(&flatten_cells_to_lines(&cells, 80));
        assert!(output.contains("❯ YOU"), "{output}");
        assert!(output.contains("╱╲ ALTAI"), "{output}");
    }

    #[test]
    fn completed_tools_render_as_compact_timeline_rows() {
        let cell = Cell::ToolNotice {
            phase: ToolNoticePhase::Result,
            content: "read src/main.rs".into(),
            tool_call_id: None,
        };
        let lines = cell_block_lines(&cell, 80);
        let output = plain(&lines);
        assert!(output.contains("✓ DONE"), "{output}");
        assert!(output.contains("read src/main.rs"), "{output}");
        assert_eq!(lines.len(), 1);
    }
}
