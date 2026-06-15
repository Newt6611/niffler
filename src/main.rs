use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use niffler::app::App;
use niffler::config;
use niffler::editor;
use niffler::filesystem;
use niffler::mode::{DeleteTarget, Mode, PickerState, PickerTarget, RenameTarget, Screen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

fn main() -> io::Result<()> {
    let root = config::data_root();
    let mut app = App::new(root)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| niffler::tui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            handle_key(terminal, app, key)?;
        }
    }
    Ok(())
}

fn handle_key(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    if app.screen == Screen::Projects
        && app.mode == Mode::Normal
        && matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
    {
        return app.switch_tab();
    }

    match app.screen {
        Screen::Projects => handle_project_key(app, key),
        Screen::Board => handle_board_key(terminal, app, key),
    }
}

fn handle_project_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    if app.mode == Mode::Help {
        return handle_help_key(app, key);
    }

    if let Mode::CreateProject { input } = app.mode.clone() {
        return handle_create_project_key(app, key, input);
    }
    if let Mode::Rename { target, input } = app.mode.clone() {
        return handle_rename_key(app, key, target, input);
    }
    if let Mode::ConfirmDelete { target } = app.mode.clone() {
        return handle_confirm_delete_key(app, key, target);
    }

    match key.code {
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_project_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_project_selection(-1),
        KeyCode::Enter => app.open_selected_project()?,
        KeyCode::Char('n') => {
            app.mode = Mode::CreateProject {
                input: String::new(),
            }
        }
        KeyCode::Char('d') => {
            app.mode = Mode::ConfirmDelete {
                target: DeleteTarget::Board,
            }
        }
        KeyCode::Char('r') => {
            let input = app
                .projects
                .get(app.selected_project)
                .map(|project| project.name.clone())
                .unwrap_or_default();
            app.mode = Mode::Rename {
                target: RenameTarget::Board,
                input,
            };
        }
        _ => {}
    }
    Ok(())
}

fn handle_board_key(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    match app.mode.clone() {
        Mode::Normal => handle_normal_key(terminal, app, key),
        Mode::ConfirmDelete { target } => handle_confirm_delete_key(app, key, target),
        Mode::CreateProject { .. } => {
            app.mode = Mode::Normal;
            Ok(())
        }
        Mode::CreateList { input } => handle_create_list_key(app, key, input),
        Mode::Add { input } => handle_add_key(app, key, input),
        Mode::Rename { target, input } => handle_rename_key(app, key, target, input),
        Mode::Move {
            target_list,
            target_position,
        } => handle_move_key(app, key, target_list, target_position),
        Mode::MoveList { target_position } => handle_move_list_key(app, key, target_position),
        Mode::Help => handle_help_key(app, key),
        Mode::Picker(picker) => handle_picker_key(app, key, picker),
    }
}

fn handle_help_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    if matches!(
        key.code,
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
    ) {
        app.mode = Mode::Normal;
    }
    Ok(())
}

fn handle_confirm_delete_key(app: &mut App, key: KeyEvent, target: DeleteTarget) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_delete(target)?;
            app.mode = Mode::Normal;
        }
        _ => app.mode = Mode::ConfirmDelete { target },
    }
    Ok(())
}

fn handle_create_project_key(app: &mut App, key: KeyEvent, mut input: String) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::Normal,
        KeyCode::Enter => {
            if !input.trim().is_empty() {
                app.create_project(&input)?;
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            input.pop();
            app.mode = Mode::CreateProject { input };
        }
        KeyCode::Char(ch) => {
            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                input.push(ch);
                app.mode = Mode::CreateProject { input };
            }
        }
        _ => app.mode = Mode::CreateProject { input },
    }
    Ok(())
}

fn handle_normal_key(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            app.screen = Screen::Projects;
            app.board = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => app.move_card_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_card_selection(-1),
        KeyCode::Char('h') | KeyCode::Left => app.move_list_selection(-1),
        KeyCode::Char('l') | KeyCode::Right => app.move_list_selection(1),
        KeyCode::Tab => app.move_list_selection(1),
        KeyCode::BackTab => app.move_list_selection(-1),
        KeyCode::Char('p') => app.toggle_preview()?,
        KeyCode::Char('C') => start_list_color_picker(app),
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('n') => start_new_card(app),
        KeyCode::Char('N') => {
            app.mode = Mode::CreateList {
                input: String::new(),
            }
        }
        KeyCode::Char('m') => {
            start_move_card(app);
        }
        KeyCode::Char('M') => {
            app.mode = Mode::MoveList {
                target_position: app.selected_list,
            };
        }
        KeyCode::Char('d') if app.selected_card_path().is_some() => {
            app.mode = Mode::ConfirmDelete {
                target: DeleteTarget::Card,
            }
        }
        KeyCode::Char('D') => {
            app.mode = Mode::ConfirmDelete {
                target: DeleteTarget::List,
            }
        }
        KeyCode::Char('r') if app.selected_card_path().is_some() => {
            let input = app
                .board
                .as_ref()
                .and_then(|board| board.lists.get(app.selected_list))
                .and_then(|list| {
                    list.cards
                        .get(*app.selected_cards.get(app.selected_list).unwrap_or(&0))
                })
                .map(|card| card.title.clone())
                .unwrap_or_default();
            app.mode = Mode::Rename {
                target: RenameTarget::Card,
                input,
            };
        }
        KeyCode::Char('R') => {
            let input = app
                .board
                .as_ref()
                .and_then(|board| board.lists.get(app.selected_list))
                .map(|list| list.name.clone())
                .unwrap_or_default();
            app.mode = Mode::Rename {
                target: RenameTarget::List,
                input,
            };
        }
        KeyCode::Char('e') => edit_selected_card(terminal, app)?,
        _ => {}
    }
    Ok(())
}

fn start_new_card(app: &mut App) {
    if app
        .board
        .as_ref()
        .is_none_or(|board| board.lists.is_empty())
    {
        app.mode = Mode::CreateList {
            input: String::new(),
        };
        return;
    }

    app.mode = Mode::Add {
        input: String::new(),
    };
}

fn start_move_card(app: &mut App) {
    if app.selected_card_path().is_none() {
        return;
    }

    app.mode = Mode::Move {
        target_list: app.selected_list,
        target_position: app.default_move_target_position(),
    };
}

fn start_list_color_picker(app: &mut App) {
    if let Some(picker) = app.list_border_color_picker() {
        app.mode = Mode::Picker(picker);
    } else {
        app.status = "No list selected".to_string();
    }
}

fn handle_create_list_key(app: &mut App, key: KeyEvent, mut input: String) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Enter => {
            if !input.trim().is_empty() {
                app.create_list(&input)?;
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            input.pop();
            app.mode = Mode::CreateList { input };
        }
        KeyCode::Char(ch) => {
            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                input.push(ch);
                app.mode = Mode::CreateList { input };
            }
        }
        _ => app.mode = Mode::CreateList { input },
    }
    Ok(())
}

fn handle_add_key(app: &mut App, key: KeyEvent, mut input: String) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Enter => {
            if !input.trim().is_empty() {
                app.create_card(&input)?;
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            input.pop();
            app.mode = Mode::Add { input };
        }
        KeyCode::Char(ch) => {
            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                input.push(ch);
                app.mode = Mode::Add { input };
            }
        }
        _ => app.mode = Mode::Add { input },
    }
    Ok(())
}

fn handle_rename_key(
    app: &mut App,
    key: KeyEvent,
    target: RenameTarget,
    mut input: String,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Enter => {
            if !input.trim().is_empty() {
                match target {
                    RenameTarget::Board => app.rename_selected_project(&input)?,
                    RenameTarget::List => app.rename_selected_list(&input)?,
                    RenameTarget::Card => app.rename_selected_card(&input)?,
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            input.pop();
            app.mode = Mode::Rename { target, input };
        }
        KeyCode::Char(ch) => {
            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                input.push(ch);
                app.mode = Mode::Rename { target, input };
            } else {
                app.mode = Mode::Rename { target, input };
            }
        }
        _ => app.mode = Mode::Rename { target, input },
    }
    Ok(())
}

fn handle_move_key(
    app: &mut App,
    key: KeyEvent,
    mut target_list: usize,
    mut target_position: usize,
) -> io::Result<()> {
    let list_count = app
        .board
        .as_ref()
        .map(|board| board.lists.len())
        .unwrap_or(0);
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Char('h') | KeyCode::Left => {
            if list_count > 0 {
                target_list = (target_list + list_count - 1) % list_count;
                target_position = app.move_target_position(target_list, target_position, 0);
            }
            app.mode = Mode::Move {
                target_list,
                target_position,
            };
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if list_count > 0 {
                target_list = (target_list + 1) % list_count;
                target_position = app.move_target_position(target_list, target_position, 0);
            }
            app.mode = Mode::Move {
                target_list,
                target_position,
            };
        }
        KeyCode::Char('k') | KeyCode::Up => {
            target_position = app.move_target_position(target_list, target_position, -1);
            app.mode = Mode::Move {
                target_list,
                target_position,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            target_position = app.move_target_position(target_list, target_position, 1);
            app.mode = Mode::Move {
                target_list,
                target_position,
            };
        }
        KeyCode::Enter => {
            app.move_selected_card_to(target_list, target_position)?;
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Move {
                target_list,
                target_position,
            }
        }
    }
    Ok(())
}

fn handle_move_list_key(
    app: &mut App,
    key: KeyEvent,
    mut target_position: usize,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Char('h') | KeyCode::Left => {
            target_position = app.move_list_target_position(target_position, -1);
            app.mode = Mode::MoveList { target_position };
        }
        KeyCode::Char('l') | KeyCode::Right => {
            target_position = app.move_list_target_position(target_position, 1);
            app.mode = Mode::MoveList { target_position };
        }
        KeyCode::Enter => {
            app.move_selected_list_to(target_position)?;
            app.mode = Mode::Normal;
        }
        _ => app.mode = Mode::MoveList { target_position },
    }
    Ok(())
}

fn handle_picker_key(app: &mut App, key: KeyEvent, mut picker: PickerState) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::Normal,
        KeyCode::Char('k') | KeyCode::Up => {
            if picker.selected > 0 {
                picker.selected -= 1;
            }
            app.mode = Mode::Picker(picker);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !picker.options.is_empty() {
                picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
            }
            app.mode = Mode::Picker(picker);
        }
        KeyCode::Enter => {
            if let Some(option) = picker.options.get(picker.selected) {
                match picker.target {
                    PickerTarget::ListBorderColor { list_index } => {
                        app.set_list_border_color(list_index, &option.value)?;
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        _ => app.mode = Mode::Picker(picker),
    }
    Ok(())
}

fn edit_selected_card(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let Some(path) = app.selected_card_path() else {
        app.status = "No card selected".to_string();
        return Ok(());
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let edit_result = editor::edit_card(&path);

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    match edit_result {
        Ok(()) => {
            filesystem::touch_card_updated_at(&path)?;
            app.status = "Edited card".to_string();
            app.reload_board()?;
        }
        Err(error) => app.status = format!("Editor failed: {error}"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use niffler::board::{
        Board, List, default_app_config, default_color_options, default_theme_colors,
    };
    use std::path::PathBuf;

    fn app_with_empty_board() -> App {
        App {
            root: PathBuf::from("/tmp"),
            config: default_app_config(),
            screen: Screen::Board,
            mode: Mode::Normal,
            show_preview: false,
            projects: Vec::new(),
            board: Some(Board {
                name: "empty".to_string(),
                path: PathBuf::from("/tmp/empty"),
                theme: default_theme_colors(),
                colors: default_color_options(),
                lists: Vec::new(),
            }),
            project_preview: None,
            selected_project: 0,
            selected_list: 0,
            selected_cards: Vec::new(),
            status: String::new(),
            should_quit: false,
        }
    }

    #[test]
    fn new_card_on_board_without_lists_opens_create_list_popup() {
        let mut app = app_with_empty_board();

        start_new_card(&mut app);

        assert_eq!(
            app.mode,
            Mode::CreateList {
                input: String::new(),
            }
        );
    }

    #[test]
    fn new_card_on_board_with_lists_opens_create_card_popup() {
        let mut app = app_with_empty_board();
        app.board = Some(Board {
            name: "ready".to_string(),
            path: PathBuf::from("/tmp/ready"),
            theme: default_theme_colors(),
            colors: default_color_options(),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("/tmp/ready/todo"),
                cards: Vec::new(),
                border_color: None,
            }],
        });
        app.selected_cards = vec![0];

        start_new_card(&mut app);

        assert_eq!(
            app.mode,
            Mode::Add {
                input: String::new()
            }
        );
    }

    #[test]
    fn uppercase_c_opens_list_border_color_picker() {
        let mut app = app_with_empty_board();
        app.board = Some(Board {
            name: "ready".to_string(),
            path: PathBuf::from("/tmp/ready"),
            theme: default_theme_colors(),
            colors: default_color_options(),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("/tmp/ready/todo"),
                cards: Vec::new(),
                border_color: Some("#22c55e".to_string()),
            }],
        });
        app.selected_cards = vec![0];

        start_list_color_picker(&mut app);

        let Mode::Picker(picker) = app.mode else {
            panic!("expected picker mode");
        };
        assert_eq!(picker.title, "List Border Color");
        assert_eq!(
            picker.target,
            PickerTarget::ListBorderColor { list_index: 0 }
        );
        assert_eq!(picker.options[picker.selected].value, "#22c55e");
    }

    #[test]
    fn q_closes_list_border_color_picker() {
        let mut app = app_with_empty_board();
        app.board = Some(Board {
            name: "ready".to_string(),
            path: PathBuf::from("/tmp/ready"),
            theme: default_theme_colors(),
            colors: default_color_options(),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("/tmp/ready/todo"),
                cards: Vec::new(),
                border_color: None,
            }],
        });
        app.selected_cards = vec![0];
        start_list_color_picker(&mut app);
        let Mode::Picker(picker) = app.mode.clone() else {
            panic!("expected picker mode");
        };

        handle_picker_key(&mut app, KeyEvent::from(KeyCode::Char('q')), picker).unwrap();

        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn move_card_on_empty_list_stays_normal() {
        let mut app = app_with_empty_board();
        app.board = Some(Board {
            name: "ready".to_string(),
            path: PathBuf::from("/tmp/ready"),
            theme: default_theme_colors(),
            colors: default_color_options(),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("/tmp/ready/todo"),
                cards: Vec::new(),
                border_color: None,
            }],
        });
        app.selected_cards = vec![0];

        start_move_card(&mut app);

        assert_eq!(app.mode, Mode::Normal);
    }
}
