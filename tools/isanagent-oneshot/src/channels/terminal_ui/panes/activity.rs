//! Persistent wide-layout run overview.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::channels::terminal_ui::text_format::truncate_chars_display;
use crate::channels::terminal_ui::{App, Theme, ToolNoticePhase};

pub fn activity_overview_paragraph(app: &App, area: Rect) -> Paragraph<'static> {
    let width = area.width.saturating_sub(2).max(8) as usize;
    let running_agents = app
        .agent_tasks
        .iter()
        .filter(|task| !task.status.is_terminal())
        .count();

    let (run_icon, run_label, run_style) = if app.pending_approval {
        ("!".to_string(), "APPROVAL REQUIRED", Theme::tool_call())
    } else if app.thinking {
        (
            app.get_spinner_frame().to_string(),
            "RUNNING",
            Theme::active(),
        )
    } else {
        ("·".to_string(), "IDLE", Theme::dim())
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(" ACTIVE RUN", Theme::dim())),
        Line::from(vec![
            Span::styled(format!(" {run_icon} "), run_style),
            Span::styled(run_label.to_string(), run_style),
        ]),
        Line::from(Span::styled(
            format!(
                " {}",
                truncate_chars_display(&app.status_model, width.saturating_sub(1))
            ),
            Theme::text(),
        )),
        Line::from(vec![
            Span::styled(" mode  ", Theme::dim()),
            Span::styled(app.status_permission.clone(), Theme::active()),
        ]),
        Line::from(""),
        Line::from(Span::styled(" ACTIVITY", Theme::dim())),
    ];

    if let Some(active) = app.active_tool_line.as_deref() {
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", app.get_spinner_frame()), Theme::tool_call()),
            Span::styled(
                truncate_chars_display(active, width.saturating_sub(3)),
                Theme::text(),
            ),
        ]));
    } else if let Some(entry) = app.tool_rail.last() {
        let (icon, style) = match entry.phase {
            ToolNoticePhase::Pending | ToolNoticePhase::Call => ("●", Theme::tool_pending()),
            ToolNoticePhase::Result => ("✓", Theme::tool_done()),
            ToolNoticePhase::Failed => ("✕", Theme::error()),
            ToolNoticePhase::Other => ("›", Theme::dim()),
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {icon} "), style),
            Span::styled(
                truncate_chars_display(&entry.summary, width.saturating_sub(3)),
                Theme::text(),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            " · No tool activity",
            Theme::dim(),
        )));
    }

    lines.extend([
        Line::from(""),
        Line::from(Span::styled(" WORKSPACE", Theme::dim())),
        metric_line("todos", app.todos_count.to_string()),
        metric_line("jobs", app.jobs_strip.len().to_string()),
        metric_line("agents", running_agents.to_string()),
        metric_line("cells", app.cells.len().to_string()),
        Line::from(""),
        Line::from(Span::styled(" Tab cycles detailed panes", Theme::dim())),
    ]);

    Paragraph::new(Text::from(lines)).block(
        Block::default()
            .borders(Borders::LEFT)
            .title(Line::from(vec![
                Span::styled(" RUN ", Theme::active()),
                Span::styled(" TOOLS  AGENTS ", Theme::dim()),
            ]))
            .border_style(Theme::border())
            .style(Theme::raised()),
    )
}

fn metric_line(label: &'static str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {label:<8}"), Theme::dim()),
        Span::styled(value, Theme::text()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;

    #[test]
    fn wide_activity_rail_exposes_run_and_workspace_state() {
        let mut app = App::new();
        app.status_model = "anthropic/claude-sonnet".into();
        app.status_permission = "plan".into();
        app.todos_count = 3;
        app.thinking = true;

        let area = Rect::new(0, 0, 40, 20);
        let mut buffer = Buffer::empty(area);
        activity_overview_paragraph(&app, area).render(area, &mut buffer);
        let output = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("ACTIVE RUN"), "{output}");
        assert!(output.contains("RUNNING"), "{output}");
        assert!(output.contains("todos"), "{output}");
        assert!(output.contains('3'), "{output}");
    }
}
