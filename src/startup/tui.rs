use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    adapters::{embedded, filesystem::FilesystemStore},
    core::{
        document::{Document, DocumentSummary, Kind, Status, today},
        error::{MakespecError, Result},
        service::{self, ProjectStore},
    },
};

type Backend = CrosstermBackend<Stdout>;

struct TerminalGuard {
    terminal: Terminal<Backend>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;

        if let Err(error) = io::stdout().execute(EnterAlternateScreen) {
            let _ = terminal::disable_raw_mode();

            return Err(error.into());
        }

        let terminal = match Terminal::new(CrosstermBackend::new(io::stdout())) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = io::stdout().execute(LeaveAlternateScreen);
                let _ = terminal::disable_raw_mode();

                return Err(error.into());
            }
        };

        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
    }
}

pub(crate) struct SuspendedTerminal {
    restored: bool,
}

impl Drop for SuspendedTerminal {
    fn drop(&mut self) {
        if !self.restored {
            let _ = terminal::enable_raw_mode();
            let _ = io::stdout().execute(EnterAlternateScreen);
            self.restored = true;
        }
    }
}

pub(crate) fn suspend_terminal() -> Result<SuspendedTerminal> {
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(SuspendedTerminal { restored: false })
}

#[derive(Default)]
struct Browser {
    catalog: Vec<DocumentSummary>,
    selected: usize,
    status_message: String,
    kind_filter: Option<Kind>,
    status_filter: Option<Status>,
    detail_scroll: u16,
    document: Option<Document>,
}

pub fn run() -> Result<()> {
    let store = FilesystemStore::new(&std::env::current_dir()?)?;
    let mut guard = TerminalGuard::new()?;
    let mut browser = Browser {
        catalog: service::list(&store)?,
        ..Browser::default()
    };

    loop {
        let visible = browser.visible();
        browser.load_selection(&store, &visible)?;
        guard.terminal.draw(|frame| browser.draw(frame, &visible))?;

        if !event::poll(Duration::from_secs(30))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Up if browser.selected > 0 => browser.selected -= 1,
            KeyCode::Down if browser.selected + 1 < visible.len() => browser.selected += 1,
            KeyCode::PageUp => browser.detail_scroll = browser.detail_scroll.saturating_sub(10),
            KeyCode::PageDown => browser.detail_scroll = browser.detail_scroll.saturating_add(10),
            KeyCode::Char('k') => {
                browser.kind_filter = match browser.kind_filter {
                    None => Some(Kind::Adr),
                    Some(Kind::Adr) => Some(Kind::Rfc),
                    Some(Kind::Rfc) => None,
                };
                browser.selected = 0;
                browser.detail_scroll = 0;
            }
            KeyCode::Char('s') => {
                browser.status_filter = match browser.status_filter {
                    None => Some(Status::Proposed),
                    Some(Status::Proposed) => Some(Status::Accepted),
                    Some(Status::Accepted) => Some(Status::Rejected),
                    Some(Status::Rejected) => Some(Status::Deprecated),
                    Some(Status::Deprecated) => Some(Status::Superseded),
                    Some(Status::Superseded) => None,
                };
                browser.selected = 0;
                browser.detail_scroll = 0;
            }
            KeyCode::Char('?') => {
                browser.status_message =
                    "Use u to reload, n to create, e to edit, a/r to change status.".into();
            }
            KeyCode::Char('u') => {
                browser.catalog = service::list(&store)?;
                browser.status_message = "Catalog reloaded".into();
            }
            KeyCode::Char('a') | KeyCode::Char('r') => {
                if let Some(document) = visible.get(browser.selected) {
                    let id = document.metadata.id.clone();
                    let next = if key.code == KeyCode::Char('a') {
                        Status::Accepted
                    } else {
                        Status::Rejected
                    };

                    match service::change_status(&store, &id, next, None) {
                        Ok(_) => {
                            browser.catalog = service::list(&store)?;
                            browser.status_message = format!("{id} is {next}");
                        }
                        Err(error) => browser.status_message = error.to_string(),
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(summary) = visible.get(browser.selected) {
                    let id = &summary.metadata.id;
                    let document = service::get(&store, id)?;

                    match edit_document(&store, &document) {
                        Ok(()) => {
                            browser.catalog = service::list(&store)?;
                            browser.status_message = format!("Updated {id}");
                            browser.document = None;
                        }
                        Err(error) => browser.status_message = error.to_string(),
                    }
                }
            }
            KeyCode::Char('n') => {
                if let Some((kind, title)) = create_prompt(&mut guard)? {
                    match create_document(&store, kind, title) {
                        Ok(document) => {
                            let id = document.metadata.id;
                            browser.catalog = service::list(&store)?;
                            browser.kind_filter = None;
                            browser.status_filter = None;
                            browser.selected = browser
                                .catalog
                                .iter()
                                .position(|summary| summary.metadata.id == id)
                                .unwrap_or(0);
                            browser.status_message = format!("Created {id}");
                        }
                        Err(error) => browser.status_message = error.to_string(),
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

impl Browser {
    fn visible(&self) -> Vec<DocumentSummary> {
        self.catalog
            .iter()
            .filter(|summary| {
                self.kind_filter.is_none_or(|kind| summary.kind == kind)
                    && self
                        .status_filter
                        .is_none_or(|status| summary.metadata.status == status)
            })
            .cloned()
            .collect()
    }

    fn load_selection(
        &mut self,
        store: &impl ProjectStore,
        visible: &[DocumentSummary],
    ) -> Result<()> {
        self.selected = self.selected.min(visible.len().saturating_sub(1));
        let next_id = visible
            .get(self.selected)
            .map(|summary| summary.metadata.id.as_str());
        let current_id = self
            .document
            .as_ref()
            .map(|document| document.metadata.id.as_str());

        if next_id != current_id {
            self.document = match next_id {
                Some(id) => Some(service::get(store, id)?),
                None => None,
            };
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame, visible: &[DocumentSummary]) {
        if frame.area().width < 50 || frame.area().height < 8 {
            frame.render_widget(
                Paragraph::new("Terminal too small. Resize the window, or press q to quit.")
                    .wrap(Wrap { trim: true }),
                frame.area(),
            );

            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(4), Constraint::Length(2)])
            .split(frame.area());
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(layout[0]);
        let items = visible
            .iter()
            .map(|summary| {
                ListItem::new(Line::from(format!(
                    "{} [{}] {}",
                    summary.metadata.id, summary.metadata.status, summary.metadata.title
                )))
            })
            .collect::<Vec<_>>();
        let mut list_state = ListState::default();
        list_state.select((!items.is_empty()).then_some(self.selected));

        let list = List::new(items)
            .block(Block::default().title("Makespec").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, panes[0], &mut list_state);

        let detail = visible.get(self.selected).map(|summary| {
            format!(
                "{}\nKind: {}\nStatus: {}\nDate: {}\n\n{}",
                summary.metadata.title,
                summary.kind,
                summary.metadata.status,
                summary.metadata.date,
                self.document
                    .as_ref()
                    .map(|document| sanitize_terminal_text(&document.body))
                    .unwrap_or_default()
            )
        });
        let detail =
            Paragraph::new(detail.unwrap_or_else(|| "No documents yet. Press ? for help.".into()))
                .wrap(Wrap { trim: false })
                .block(Block::default().title("Details").borders(Borders::ALL));

        frame.render_widget(detail.scroll((self.detail_scroll, 0)), panes[1]);

        let kind_label = self.kind_filter.map_or("all", Kind::as_str);
        let status_label = self.status_filter.map_or("all", Status::as_str);
        let help = Paragraph::new(format!(
            "↑/↓ move  PgUp/PgDn scroll  k kind:{kind_label}  s status:{status_label}  n new  e edit  a accept  r reject  q quit  {}",
            sanitize_terminal_text(&self.status_message)
        ));

        frame.render_widget(help, layout[1]);
    }
}

fn edit_document(store: &impl ProjectStore, document: &Document) -> Result<()> {
    let id = &document.metadata.id;
    let edited = crate::startup::cli::edit_with_editor(document)?;

    if let Err(error) = service::edit(store, id, &edited) {
        let message = match service::save_recovery(store, id, &edited) {
            Ok(path) => format!("{error}; draft saved to {}", path.display()),
            Err(recovery_error) => {
                format!("{error}; could not save recovery draft: {recovery_error}")
            }
        };

        return Err(MakespecError::Invalid(message));
    }

    Ok(())
}

fn create_document(store: &impl ProjectStore, kind: Kind, title: String) -> Result<Document> {
    let template = match kind {
        Kind::Adr => embedded::ADR_TEMPLATE,
        Kind::Rfc => embedded::RFC_TEMPLATE,
    };
    let edited = crate::startup::cli::edit_text_with_editor(template)?;

    service::create(store, kind, title, edited, &today())
}

fn sanitize_terminal_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && !matches!(character, '\n' | '\t') {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect()
}

fn create_prompt(guard: &mut TerminalGuard) -> Result<Option<(Kind, String)>> {
    let mut kind = Kind::Adr;
    let mut title = String::new();

    loop {
        guard.terminal.draw(|frame| {
            let text = Text::from(vec![
                Line::from("Create a document"),
                Line::from(""),
                Line::from(format!("Type: {} (Tab to switch)", kind)),
                Line::from(format!("Title: {title}_")),
                Line::from("Enter continues; Esc cancels."),
            ]);

            frame.render_widget(
                Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("New")),
                frame.area(),
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Tab => {
                    kind = if kind == Kind::Adr {
                        Kind::Rfc
                    } else {
                        Kind::Adr
                    };
                }
                KeyCode::Backspace => {
                    title.pop();
                }
                KeyCode::Enter if !title.trim().is_empty() => {
                    return Ok(Some((kind, title.trim().to_string())));
                }
                KeyCode::Char(character) if !character.is_control() && title.len() < 200 => {
                    title.push(character);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_is_clamped_and_cleared_when_filters_hide_all_documents() {
        let directory = tempfile::tempdir().unwrap();
        let store = FilesystemStore::new(directory.path()).unwrap();
        service::initialize(&store, "2026-11-09", &[]).unwrap();

        let mut browser = Browser {
            catalog: service::list(&store).unwrap(),
            selected: usize::MAX,
            ..Browser::default()
        };
        let visible = browser.visible();
        browser.load_selection(&store, &visible).unwrap();

        assert_eq!(browser.selected, 1);
        assert_eq!(browser.document.as_ref().unwrap().metadata.id, "RFC-0001");

        browser.kind_filter = Some(Kind::Adr);
        browser.status_filter = Some(Status::Proposed);
        let visible = browser.visible();
        browser.load_selection(&store, &visible).unwrap();

        assert!(visible.is_empty());
        assert_eq!(browser.selected, 0);
        assert!(browser.document.is_none());
    }

    #[test]
    fn terminal_text_replaces_controls_but_keeps_layout_whitespace() {
        assert_eq!(
            sanitize_terminal_text("row\nnext\t\u{1b}[2J"),
            "row\nnext\t\u{fffd}[2J"
        );
    }
}
