//! ALTAI terminal brand treatment and the empty-session welcome surface.

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{App, Theme};

const ALTAI_LOGO: &[&str] = &[
    "       ╭────╮",
    "      ╱      ╲",
    "     ╱   ╱╲   ╲",
    "    ╱   ╱  ╲   ╲",
    "   ╱___╱    ╲___╲",
];

pub const COMPACT_MARK: &str = "╱╲";

pub fn is_pristine_session(app: &App) -> bool {
    app.input.is_empty()
        && app.cells.len() == 1
        && matches!(app.cells.first(), Some(super::Cell::System { .. }))
}

pub fn welcome_paragraph(app: &App, area: Rect) -> Paragraph<'static> {
    let roomy = area.width >= 52 && area.height >= 15;
    let mut content: Vec<Line<'static>> = Vec::new();

    if roomy {
        content.extend(
            ALTAI_LOGO
                .iter()
                .map(|line| Line::from(Span::styled((*line).to_string(), Theme::active()))),
        );
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "A L T A I",
            Theme::text().add_modifier(ratatui::style::Modifier::BOLD),
        )));
    } else {
        content.push(Line::from(vec![
            Span::styled(format!("{COMPACT_MARK} "), Theme::active()),
            Span::styled(
                "ALTAI",
                Theme::text().add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Agentic work, visible from plan to result.",
        Theme::dim(),
    )));
    content.push(Line::from(""));
    if roomy {
        content.push(Line::from(vec![
            Span::styled("/init", Theme::active()),
            Span::styled(" understand project   ", Theme::dim()),
            Span::styled("/resume", Theme::active()),
            Span::styled(" continue session   ", Theme::dim()),
            Span::styled("@file", Theme::active()),
            Span::styled(" add context", Theme::dim()),
        ]));
    } else {
        content.push(Line::from(vec![
            Span::styled("/init", Theme::active()),
            Span::styled("   ", Theme::dim()),
            Span::styled("/resume", Theme::active()),
            Span::styled("   ", Theme::dim()),
            Span::styled("@file", Theme::active()),
        ]));
    }
    if roomy {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled(app.status_workspace.clone(), Theme::text()),
            Span::styled("  ·  ", Theme::dim()),
            Span::styled(app.status_model.clone(), Theme::dim()),
            Span::styled("  ·  ", Theme::dim()),
            Span::styled(app.status_permission.clone(), Theme::active()),
        ]));
    }

    let inner_height = area.height as usize;
    let top_padding = inner_height.saturating_sub(content.len()) / 2;
    let mut lines = Vec::with_capacity(top_padding + content.len());
    lines.extend((0..top_padding).map(|_| Line::from("")));
    lines.extend(content);

    Paragraph::new(Text::from(lines))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Theme::active())
                .style(Theme::panel()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;

    #[test]
    fn pristine_session_requires_only_opening_banner() {
        let mut app = App::new();
        assert!(!is_pristine_session(&app));
        app.cells.push(super::super::Cell::System {
            message: "welcome".into(),
        });
        assert!(is_pristine_session(&app));
        app.cells.push(super::super::Cell::User {
            text: "hello".into(),
        });
        assert!(!is_pristine_session(&app));
    }

    #[test]
    fn logo_fits_reference_terminal_width() {
        assert!(ALTAI_LOGO
            .iter()
            .all(|line| super::super::display_width(line) < 80));
    }

    #[test]
    fn roomy_welcome_renders_ascii_logo_and_actions() {
        let area = Rect::new(0, 0, 100, 24);
        let mut buffer = Buffer::empty(area);
        welcome_paragraph(&App::new(), area).render(area, &mut buffer);
        let output = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("╭────╮"), "{output}");
        assert!(output.contains("/resume"), "{output}");
    }

    #[test]
    fn compact_welcome_keeps_brand_visible() {
        let area = Rect::new(0, 0, 40, 10);
        let mut buffer = Buffer::empty(area);
        welcome_paragraph(&App::new(), area).render(area, &mut buffer);
        let output = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("╱╲"), "{output}");
        assert!(output.contains("ALTAI"), "{output}");
    }
}
