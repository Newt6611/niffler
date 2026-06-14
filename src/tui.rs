use crate::app::App;
use crate::mode::{DeleteTarget, Mode, RenameTarget, Screen};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Tabs, Wrap,
};
use std::ops::Range;

const HELP_CLOSE_LINE: &str = "? / Esc / q  Close help";
const ACTIVE_SELECTION: Color = Color::Rgb(218, 173, 82);
const HEADER: Color = Color::Rgb(236, 196, 91);
const SUCCESS: Color = Color::Rgb(190, 143, 66);
const INACTIVE: Color = Color::Rgb(88, 84, 76);
const UNFOCUSED_PANEL_BORDER: Color = Color::Rgb(60, 60, 60);
const TEXT: Color = Color::Rgb(229, 219, 199);
const MUTED: Color = Color::Rgb(154, 142, 122);
const SELECTED_TEXT: Color = Color::Black;
const SHELL: Color = Color::Rgb(122, 88, 42);
const PANEL: Color = Color::Rgb(151, 111, 58);
const PREVIEW: Color = Color::Rgb(176, 128, 62);
const MODAL: Color = Color::Rgb(244, 207, 104);
const MOVE_TARGET: Color = Color::Rgb(211, 151, 67);
const DANGER: Color = Color::Rgb(198, 93, 74);

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let mut shell = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(SHELL))
        .title_bottom(
            status_bar_line(&instruction_line(app))
                .right_aligned()
                .style(Style::default().fg(MUTED)),
        );
    if !app.status.is_empty() {
        shell = shell.title_bottom(
            Line::from(app.status.clone())
                .left_aligned()
                .style(Style::default().fg(INACTIVE)),
        );
    }
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(shell, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    render_header(frame, app, layout[0]);
    match app.screen {
        Screen::Projects => render_projects(frame, app, layout[1]),
        Screen::Board => render_board(frame, app, layout[1]),
    }

    if has_modal(&app.mode) {
        dim_background(frame, area);
    }

    match &app.mode {
        Mode::CreateProject { input } => {
            render_input_popup(frame, "New Board", "Board name", input, "Create", area)
        }
        Mode::CreateList { input } => {
            render_input_popup(frame, "New List", "List name", input, "Create", area)
        }
        Mode::Add { input } => {
            render_input_popup(frame, "New Card", "Card title", input, "Create", area)
        }
        Mode::Rename { target, input } => render_rename_popup(frame, *target, input, area),
        Mode::ConfirmDelete { target } => render_delete_confirmation(frame, app, *target, area),
        Mode::Help => render_help_popup(frame, app.screen, area),
        _ => {}
    }
}

fn has_modal(mode: &Mode) -> bool {
    matches!(
        mode,
        Mode::CreateProject { .. }
            | Mode::CreateList { .. }
            | Mode::Add { .. }
            | Mode::Rename { .. }
            | Mode::ConfirmDelete { .. }
            | Mode::Help
    )
}

fn dim_background(frame: &mut Frame<'_>, area: Rect) {
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            let Some(cell) = frame.buffer_mut().cell_mut((x, y)) else {
                continue;
            };
            let style = cell.style();
            cell.set_style(
                style.patch(
                    Style::default()
                        .fg(INACTIVE)
                        .bg(Color::Black)
                        .add_modifier(Modifier::DIM),
                ),
            );
        }
    }
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let board_title = app
        .board
        .as_ref()
        .map(|board| board.name.clone())
        .unwrap_or_else(|| "Board".to_string());
    let selected = match app.screen {
        Screen::Projects => 0,
        Screen::Board => 1,
    };
    let tabs = Tabs::new(vec!["Boards".to_string(), board_title])
        .select(selected)
        .divider(Span::styled(" . ", Style::default().fg(INACTIVE)))
        .style(Style::default().fg(MUTED))
        .highlight_style(
            Style::default()
                .fg(ACTIVE_SELECTION)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn render_projects(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if app.projects.is_empty() {
        let paragraph =
            Paragraph::new("No boards found. Press n to create one, or create a directory.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(INACTIVE));
        frame.render_widget(paragraph, area);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let items = app
        .projects
        .iter()
        .map(|project| {
            ListItem::new(Text::from(project.name.clone())).style(Style::default().fg(TEXT))
        })
        .collect::<Vec<_>>();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(SELECTED_TEXT)
                .bg(ACTIVE_SELECTION)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
        .block(
            Block::default()
                .title(" Boards ")
                .borders(Borders::ALL)
                .style(Style::default().fg(PANEL)),
        );
    let mut state = ListState::default().with_selected(Some(app.selected_project));
    frame.render_stateful_widget(list, layout[0], &mut state);

    let preview = app
        .project_preview
        .as_ref()
        .map(board_details_text)
        .unwrap_or_else(|| {
            Text::from(Line::styled(
                "No board selected",
                Style::default().fg(INACTIVE),
            ))
        });
    let preview = Paragraph::new(preview).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(" Board Details ")
            .borders(Borders::ALL)
            .style(Style::default().fg(PREVIEW)),
    );
    frame.render_widget(preview, layout[1]);
}

fn board_details_text(board: &crate::board::Board) -> Text<'static> {
    let card_count = board
        .lists
        .iter()
        .map(|list| list.cards.len())
        .sum::<usize>();
    let mut lines = vec![
        Line::styled(
            board.name.clone(),
            Style::default()
                .fg(ACTIVE_SELECTION)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                "{} / {}",
                count_label(board.lists.len(), "list"),
                count_label(card_count, "card")
            ),
            Style::default().fg(INACTIVE),
        ),
        Line::from(""),
    ];

    if board.lists.is_empty() {
        lines.push(Line::styled("No lists yet", Style::default().fg(INACTIVE)));
        return Text::from(lines);
    }

    for list in &board.lists {
        lines.push(Line::from(vec![
            Span::styled(
                list.name.clone(),
                Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", count_label(list.cards.len(), "card")),
                Style::default().fg(INACTIVE),
            ),
        ]));
        if list.cards.is_empty() {
            lines.push(Line::styled("  No cards", Style::default().fg(INACTIVE)));
        } else {
            lines.extend(card_titles(&list.cards));
        }
        lines.push(Line::from(""));
    }
    lines.pop();
    Text::from(lines)
}

fn card_titles(cards: &[crate::card::Card]) -> impl Iterator<Item = Line<'static>> + '_ {
    cards.iter().map(|card| {
        Line::from(vec![
            Span::raw("  - "),
            Span::styled(card.title.clone(), Style::default().fg(TEXT)),
        ])
    })
}

fn count_label(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

#[cfg(test)]
fn board_header_text(board: &crate::board::Board, mode_label: &str) -> String {
    let card_count = board
        .lists
        .iter()
        .map(|list| list.cards.len())
        .sum::<usize>();
    format!(
        "✦ {}   ·  {}   ·  {}      {}",
        board.name,
        count_label(board.lists.len(), "list"),
        count_label(card_count, "card"),
        mode_label.to_uppercase()
    )
}

fn board_header_line(board: &crate::board::Board, mode_label: &str) -> Line<'static> {
    let card_count = board
        .lists
        .iter()
        .map(|list| list.cards.len())
        .sum::<usize>();
    Line::from(vec![
        Span::styled("✦ ", Style::default().fg(HEADER)),
        Span::styled(
            board.name.clone(),
            Style::default().fg(HEADER).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ·  ", Style::default().fg(INACTIVE)),
        Span::styled(
            count_label(board.lists.len(), "list"),
            Style::default().fg(TEXT),
        ),
        Span::styled("   ·  ", Style::default().fg(INACTIVE)),
        Span::styled(count_label(card_count, "card"), Style::default().fg(TEXT)),
        Span::raw("      "),
        Span::styled(
            mode_label.to_uppercase(),
            Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn board_content_layout(area: Rect, show_preview: bool) -> (Rect, Option<Rect>) {
    if !show_preview || area.width < 4 {
        return (area, None);
    }

    let preview_width = area.width / 2;
    if preview_width == 0 {
        return (area, None);
    }

    let list_width = area.width.saturating_sub(preview_width);
    if list_width == 0 {
        return (area, None);
    }

    let list_area = Rect {
        x: area.x,
        y: area.y,
        width: list_width,
        height: area.height,
    };
    let preview_area = Rect {
        x: area.x.saturating_add(list_width),
        y: area.y,
        width: preview_width,
        height: area.height,
    };
    (list_area, Some(preview_area))
}

#[cfg(test)]
fn board_details_plain_text(board: &crate::board::Board) -> String {
    board_details_text(board)
        .lines
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_board(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(board) = &app.board else {
        return;
    };

    let content_area = if area.height >= 2 {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);
        frame.render_widget(
            Paragraph::new(board_header_line(board, app.mode.name())),
            layout[0],
        );
        layout[1]
    } else {
        area
    };

    let moving_card_title = if matches!(app.mode, Mode::Move { .. }) {
        board
            .lists
            .get(app.selected_list)
            .and_then(|list| {
                list.cards
                    .get(*app.selected_cards.get(app.selected_list).unwrap_or(&0))
            })
            .map(|card| card.title.clone())
    } else {
        None
    };

    if board.lists.is_empty() {
        let paragraph = Paragraph::new("No lists found. Press n to create one.")
            .style(Style::default().fg(INACTIVE))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Board ")
                    .style(Style::default().fg(PANEL)),
            );
        frame.render_widget(paragraph, content_area);
        return;
    }

    let (lists_area, preview_area) = board_content_layout(content_area, app.show_preview);

    let available_columns = ((lists_area.width as usize).saturating_sub(24) / 20).max(1);
    let column_count = available_columns.min(3).min(board.lists.len());

    let list_order = list_preview_order(app);
    let focus_position = match &app.mode {
        Mode::Move { target_list, .. } => list_order
            .iter()
            .position(|list_index| list_index == target_list)
            .unwrap_or(0),
        Mode::MoveList { target_position } => {
            (*target_position).min(list_order.len().saturating_sub(1))
        }
        _ => list_order
            .iter()
            .position(|list_index| *list_index == app.selected_list)
            .unwrap_or(0),
    };
    let visible_lists = visible_list_window(focus_position, column_count, board.lists.len());
    let constraints = visible_lists
        .clone()
        .map(|_| Constraint::Ratio(1, column_count as u32))
        .collect::<Vec<_>>();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(lists_area);

    let is_card_move_mode = matches!(app.mode, Mode::Move { .. });

    for (column_area, list_index) in columns
        .iter()
        .zip(list_order[visible_lists].iter().copied())
    {
        let list = &board.lists[list_index];
        let selected_card = *app.selected_cards.get(list_index).unwrap_or(&0);
        let is_selected_list = list_index == app.selected_list;
        let is_move_target =
            matches!(&app.mode, Mode::Move { target_list, .. } if *target_list == list_index);
        let is_list_move_target = matches!(app.mode, Mode::MoveList { .. }) && is_selected_list;
        let block_style = list_block_style(is_selected_list, is_move_target, is_list_move_target);
        let title = list_title(
            &list.name,
            list.cards.len(),
            is_move_target || is_list_move_target,
        );

        let mut rows = move_preview_rows(app, list_index, moving_card_title.as_deref());
        if rows.is_empty() && !is_card_move_mode {
            rows = empty_list_rows();
        }
        let selected_row = selected_row_index(&app.mode, &rows, selected_card);
        let visible_height = column_area.height.saturating_sub(2) as usize;
        let card_start = selected_row.saturating_sub(visible_height.saturating_sub(1));
        let card_end = (card_start + visible_height).min(rows.len());
        let items = rows[card_start..card_end]
            .iter()
            .enumerate()
            .map(|(offset, row)| {
                let row_index = card_start + offset;
                let selected =
                    is_row_selected(&app.mode, row, is_selected_list, row_index, selected_card);
                let style = card_row_style(
                    row,
                    selected,
                    is_selected_list,
                    is_move_target || is_list_move_target,
                );
                ListItem::new(row.label()).style(style)
            })
            .collect::<Vec<_>>();

        let widget = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(block_style),
        );
        frame.render_widget(widget, *column_area);
    }

    if let Some(preview_area) = preview_area
        && preview_area.width > 0
    {
        render_preview_panel(frame, app, preview_area);
    }
}

fn visible_list_window(
    focus_position: usize,
    column_count: usize,
    list_count: usize,
) -> Range<usize> {
    if column_count == 0 || list_count == 0 {
        return 0..0;
    }

    let visible_count = column_count.min(list_count);
    let focus_position = focus_position.min(list_count.saturating_sub(1));
    let start = focus_position
        .saturating_sub(visible_count.saturating_sub(1))
        .min(list_count - visible_count);
    start..start + visible_count
}

fn list_preview_order(app: &App) -> Vec<usize> {
    let Some(board) = &app.board else {
        return Vec::new();
    };
    let mut order = (0..board.lists.len()).collect::<Vec<_>>();
    let Mode::MoveList { target_position } = app.mode else {
        return order;
    };
    if app.selected_list >= order.len() {
        return order;
    }

    let list = order.remove(app.selected_list);
    let target_position = target_position.min(order.len());
    order.insert(target_position, list);
    order
}

#[derive(Debug, PartialEq)]
enum BoardRow<'a> {
    Card(&'a str),
    MoveMarker(String),
    Placeholder(&'a str),
}

impl BoardRow<'_> {
    fn label(&self) -> String {
        match self {
            Self::Card(title) => (*title).to_string(),
            Self::MoveMarker(title) => format!("-> {title}"),
            Self::Placeholder(title) => (*title).to_string(),
        }
    }

    fn is_selectable(&self) -> bool {
        !matches!(self, Self::Placeholder(_))
    }
}

fn list_title(name: &str, card_count: usize, is_target: bool) -> String {
    let prefix = if is_target { " -> " } else { " " };
    format!("{prefix}{} [{}] ", name.to_uppercase(), card_count)
}

fn list_block_style(
    is_selected_list: bool,
    is_move_target: bool,
    is_list_move_target: bool,
) -> Style {
    if is_move_target || is_list_move_target {
        Style::default()
            .fg(MOVE_TARGET)
            .add_modifier(Modifier::BOLD)
    } else if is_selected_list {
        Style::default().fg(HEADER).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(UNFOCUSED_PANEL_BORDER)
    }
}

fn card_row_style(
    row: &BoardRow<'_>,
    selected: bool,
    is_selected_list: bool,
    is_move_target_or_list: bool,
) -> Style {
    if selected {
        Style::default()
            .fg(SELECTED_TEXT)
            .bg(if is_move_target_or_list {
                MOVE_TARGET
            } else {
                ACTIVE_SELECTION
            })
            .add_modifier(Modifier::BOLD)
    } else if matches!(row, BoardRow::Placeholder(_)) {
        Style::default().fg(INACTIVE)
    } else if is_selected_list {
        Style::default().fg(TEXT)
    } else {
        Style::default().fg(MUTED)
    }
}

fn selected_row_index(mode: &Mode, rows: &[BoardRow<'_>], selected_card: usize) -> usize {
    let move_marker = rows
        .iter()
        .position(|row| matches!(row, BoardRow::MoveMarker(_)));

    if let Some(index) = move_marker {
        return index;
    }

    match mode {
        Mode::MoveList { .. } => 0,
        _ => selected_card.min(rows.len().saturating_sub(1)),
    }
}

fn is_row_selected(
    mode: &Mode,
    row: &BoardRow<'_>,
    is_selected_list: bool,
    row_index: usize,
    selected_card: usize,
) -> bool {
    match mode {
        Mode::Move { .. } => matches!(row, BoardRow::MoveMarker(_)),
        Mode::MoveList { .. } => false,
        _ => row.is_selectable() && is_selected_list && row_index == selected_card,
    }
}

fn empty_list_labels() -> Vec<&'static str> {
    vec!["No cards", "Press n to create a card"]
}

fn empty_list_rows() -> Vec<BoardRow<'static>> {
    empty_list_labels()
        .into_iter()
        .map(BoardRow::Placeholder)
        .collect()
}

fn move_preview_rows<'a>(
    app: &'a App,
    list_index: usize,
    moving_card_title: Option<&str>,
) -> Vec<BoardRow<'a>> {
    let Some(board) = &app.board else {
        return Vec::new();
    };
    let Some(list) = board.lists.get(list_index) else {
        return Vec::new();
    };
    let Mode::Move {
        target_list,
        target_position,
    } = app.mode
    else {
        return list
            .cards
            .iter()
            .map(|card| BoardRow::Card(card.title.as_str()))
            .collect();
    };

    let source_card = app
        .selected_cards
        .get(app.selected_list)
        .copied()
        .unwrap_or(0);
    let moving_title = moving_card_title.unwrap_or("card").to_string();
    let mut rows = Vec::new();
    let mut insertion_index = 0;
    let mut inserted_marker = false;

    for (card_index, card) in list.cards.iter().enumerate() {
        if list_index == target_list && !inserted_marker && insertion_index == target_position {
            rows.push(BoardRow::MoveMarker(moving_title.clone()));
            inserted_marker = true;
        }
        if !(list_index == app.selected_list && card_index == source_card) {
            rows.push(BoardRow::Card(card.title.as_str()));
            insertion_index += 1;
        }
    }

    if list_index == target_list && !inserted_marker && insertion_index <= target_position {
        rows.push(BoardRow::MoveMarker(moving_title));
    }
    rows
}

fn instruction_line(app: &App) -> String {
    match app.screen {
        Screen::Projects => home_footer_text(&app.mode),
        Screen::Board => board_footer_text(&app.mode, app.show_preview),
    }
}

fn home_footer_text(mode: &Mode) -> String {
    match mode {
        Mode::Normal => {
            "[NORMAL] [Tab] Switch [j/k] Move [Enter] Open [n] New [r] Rename [?] Help [q] Quit"
                .to_string()
        }
        Mode::Help => "[HELP] [?/Esc/q] Close".to_string(),
        _ => instruction_line_for_modal(mode),
    }
}

fn board_footer_text(mode: &Mode, show_preview: bool) -> String {
    match mode {
        Mode::Normal => {
            let preview = if show_preview {
                "Hide Preview"
            } else {
                "Preview"
            };
            format!(
                "[NORMAL] [Tab] List [j/k] Move [n] New [m] Move [p] {preview} [?] Help [q] Quit"
            )
        }
        Mode::Help => "[HELP] [?/Esc/q] Close".to_string(),
        _ => instruction_line_for_modal(mode),
    }
}

fn instruction_line_for_modal(mode: &Mode) -> String {
    match mode {
        Mode::CreateProject { .. } | Mode::CreateList { .. } | Mode::Add { .. } => {
            "[CREATE] [Enter] Create [Esc] Cancel".to_string()
        }
        Mode::Rename { .. } => "[RENAME] [Enter] Rename [Esc] Cancel".to_string(),
        Mode::ConfirmDelete { .. } => "[CONFIRM] [y/Enter] Confirm [n/Esc] Cancel".to_string(),
        Mode::Move { .. } => {
            "[MOVE CARD] [h/l] List [j/k] Position [Enter] Move [Esc] Cancel".to_string()
        }
        Mode::MoveList { .. } => "[MOVE LIST] [h/l] Position [Enter] Move [Esc] Cancel".to_string(),
        Mode::Normal | Mode::Help => unreachable!(),
    }
}

fn status_bar_line(text: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut plain = String::new();

    while let Some(ch) = chars.next() {
        if ch == '[' {
            if !plain.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut plain),
                    Style::default().fg(MUTED),
                ));
            }
            let mut key = String::from("[");
            for next in chars.by_ref() {
                key.push(next);
                if next == ']' {
                    break;
                }
            }
            spans.push(Span::styled(
                key,
                Style::default()
                    .fg(SELECTED_TEXT)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            plain.push(ch);
        }
    }
    if !plain.is_empty() {
        spans.push(Span::styled(plain, Style::default().fg(MUTED)));
    }

    Line::from(spans)
}

fn help_text(screen: Screen) -> String {
    match screen {
        Screen::Projects => [
            "Keyboard Shortcuts",
            "Navigation        Boards",
            "j/k Up/Down       Enter Open",
            "Tab Switch        n New Board",
            "                  r Rename",
            "                  d Delete",
            "Help",
            HELP_CLOSE_LINE,
        ]
        .join("\n"),
        Screen::Board => [
            "Keyboard Shortcuts",
            HELP_CLOSE_LINE,
            "",
            "Navigation          Cards               Lists",
            "",
            "j/k Move            n New Card          N New List",
            "Up/Down Move        e Edit              M Move List",
            "Tab/h/l List        m Move              R Rename",
            "Left/Right List     r Rename            D Delete",
            "q Boards            d Delete",
            "p Preview",
            "",
            "Help",
            "",
            HELP_CLOSE_LINE,
        ]
        .join("\n"),
    }
}

fn rename_label(target: RenameTarget) -> &'static str {
    match target {
        RenameTarget::Board => "Board",
        RenameTarget::List => "List",
        RenameTarget::Card => "Card",
    }
}

fn render_input_popup(
    frame: &mut Frame<'_>,
    title: &str,
    label: &str,
    input: &str,
    action: &str,
    area: Rect,
) {
    let popup = centered_rect(64, 40, area);
    frame.render_widget(Clear, popup);
    let paragraph = Paragraph::new(input_popup_text(label, input, action))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1))
                .style(Style::default().fg(MODAL)),
        );
    frame.render_widget(paragraph, popup);
}

fn input_popup_text(label: &str, input: &str, action: &str) -> String {
    format!("{label}\n\n{input}\n\nEnter {action}  Esc Cancel")
}

fn render_rename_popup(frame: &mut Frame<'_>, target: RenameTarget, input: &str, area: Rect) {
    let popup = centered_rect(64, 40, area);
    frame.render_widget(Clear, popup);
    let paragraph = Paragraph::new(rename_popup_text(target, input))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Rename ")
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1))
                .style(Style::default().fg(MODAL)),
        );
    frame.render_widget(paragraph, popup);
}

fn rename_popup_text(target: RenameTarget, input: &str) -> String {
    format!(
        "Rename {}\n\n{}\n\nEnter Rename  Esc Cancel",
        rename_label(target),
        input
    )
}

fn render_delete_confirmation(frame: &mut Frame<'_>, app: &App, target: DeleteTarget, area: Rect) {
    let popup = centered_rect(62, 34, area);
    frame.render_widget(Clear, popup);
    let target_name = delete_target_name(app, target);
    let text = match target {
        DeleteTarget::Board => {
            format!("Delete board \"{target_name}\"?\n\nThis removes all lists and cards.")
        }
        DeleteTarget::List => {
            format!("Delete list \"{target_name}\"?\n\nThis removes all cards in the list.")
        }
        DeleteTarget::Card => format!("Delete card \"{target_name}\"?"),
    };
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1))
                .style(Style::default().fg(DANGER)),
        );
    frame.render_widget(paragraph, popup);
}

fn render_help_popup(frame: &mut Frame<'_>, screen: Screen, area: Rect) {
    let popup = help_popup_area(area, help_text(screen).as_str());
    frame.render_widget(Clear, popup);
    let paragraph = Paragraph::new(help_text(screen))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Keyboard Shortcuts ")
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1))
                .style(Style::default().fg(MODAL)),
        );
    frame.render_widget(paragraph, popup);
}

fn delete_target_name(app: &App, target: DeleteTarget) -> String {
    match target {
        DeleteTarget::Board => app
            .projects
            .get(app.selected_project)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "selected board".to_string()),
        DeleteTarget::List => app
            .board
            .as_ref()
            .and_then(|board| board.lists.get(app.selected_list))
            .map(|list| list.name.clone())
            .unwrap_or_else(|| "selected list".to_string()),
        DeleteTarget::Card => app
            .board
            .as_ref()
            .and_then(|board| board.lists.get(app.selected_list))
            .and_then(|list| {
                list.cards
                    .get(*app.selected_cards.get(app.selected_list).unwrap_or(&0))
            })
            .map(|card| card.title.clone())
            .unwrap_or_else(|| "selected card".to_string()),
    }
}

fn render_preview_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let content = app
        .selected_card_content()
        .unwrap_or_else(|| "No card selected".to_string());
    let paragraph = Paragraph::new(content).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(" Preview ")
            .borders(Borders::ALL)
            .style(Style::default().fg(PREVIEW)),
    );
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
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
        .split(vertical[1])[1]
}

fn help_popup_area(area: Rect, text: &str) -> Rect {
    if area.width < 2 || area.height < 2 {
        return area;
    }

    let title_len = " Keyboard Shortcuts ".len() as u16;
    let content_width = text
        .lines()
        .map(|line| line.chars().count() as u16)
        .max()
        .unwrap_or(0)
        .max(10);
    let width = content_width
        .max(title_len)
        .saturating_add(6)
        .min(area.width.saturating_sub(2));
    let inner_width = width.saturating_sub(6).max(1);
    let content_lines = wrapped_text_lines(text, inner_width);
    let height = (content_lines.saturating_add(4)).min(area.height);

    let x = area.x + (area.width.saturating_sub(width) / 2);
    let y = area.y + (area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width, height)
}

fn wrapped_text_lines(text: &str, inner_width: u16) -> u16 {
    let max_width = inner_width.max(1) as usize;
    if text.is_empty() {
        return 1;
    }
    let mut lines = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            lines += 1;
            continue;
        }
        let mut current = 0usize;
        for word in line.split_whitespace() {
            let word_len = word.len();
            if word_len > max_width {
                if current > 0 {
                    lines += 1;
                }
                lines += word_len / max_width;
                if word_len % max_width == 0 {
                    current = 0;
                } else {
                    current = word_len % max_width;
                }
                continue;
            }

            if current == 0 {
                current = word_len;
                continue;
            }
            if current + 1 + word_len <= max_width {
                current += 1 + word_len;
            } else {
                lines += 1;
                current = word_len;
            }
        }
        if current > 0 || line.trim().is_empty() {
            lines += 1;
        }
    }
    lines as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, List, Project};
    use crate::card::Card;
    use crate::{app::App, mode::Screen};
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};
    use std::path::PathBuf;

    fn sample_board() -> Board {
        Board {
            name: "strike".to_string(),
            path: PathBuf::from("strike"),
            lists: vec![
                List {
                    name: "TODO".to_string(),
                    path: PathBuf::from("todo"),
                    cards: vec![
                        card("first task", "task1.md"),
                        card("second task", "task2.md"),
                    ],
                },
                List {
                    name: "DONE".to_string(),
                    path: PathBuf::from("done"),
                    cards: Vec::new(),
                },
            ],
        }
    }

    fn sample_projects_app(mode: Mode) -> App {
        App {
            root: PathBuf::from("/tmp"),
            screen: Screen::Projects,
            mode,
            show_preview: false,
            projects: vec![
                Project {
                    name: "Work".to_string(),
                    path: PathBuf::from("/tmp/work"),
                },
                Project {
                    name: "Play".to_string(),
                    path: PathBuf::from("/tmp/play"),
                },
            ],
            board: None,
            project_preview: Some(sample_board()),
            selected_project: 0,
            selected_list: 0,
            selected_cards: vec![0],
            status: String::new(),
            should_quit: false,
        }
    }

    fn sample_board_app(show_preview: bool) -> App {
        let board = sample_board();
        App {
            root: PathBuf::from("/tmp"),
            screen: Screen::Board,
            mode: Mode::Normal,
            show_preview,
            projects: Vec::new(),
            board: Some(board),
            project_preview: None,
            selected_project: 0,
            selected_list: 0,
            selected_cards: vec![0, 0],
            status: String::new(),
            should_quit: false,
        }
    }

    fn render_tui_buffer(app: &App, area: Rect) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();

        terminal.backend().buffer().clone()
    }

    fn render_tui_lines(app: &App, area: Rect) -> Vec<String> {
        let buffer = render_tui_buffer(app, area);
        (area.top()..area.bottom())
            .map(|y| {
                let mut line = String::new();
                for x in area.left()..area.right() {
                    let symbol = buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" ");
                    line.push_str(symbol);
                }
                line
            })
            .collect()
    }

    fn render_tui_lines_in_rect(buffer: &ratatui::buffer::Buffer, rect: Rect) -> Vec<String> {
        (rect.top()..rect.bottom())
            .map(|y| {
                let mut line = String::new();
                for x in rect.left()..rect.right() {
                    let symbol = buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" ");
                    line.push_str(symbol);
                }
                line
            })
            .collect()
    }

    fn has_dimmed_background_visible(
        buffer: &ratatui::buffer::Buffer,
        area: Rect,
        popup: Rect,
    ) -> bool {
        let mut found_non_space = false;
        let mut found_dimmed = false;
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let inside_popup = x >= popup.left()
                    && x < popup.right()
                    && y >= popup.top()
                    && y < popup.bottom();
                if inside_popup {
                    continue;
                }
                let Some(cell) = buffer.cell((x, y)) else {
                    continue;
                };
                if cell.symbol() == " " {
                    continue;
                }
                found_non_space = true;
                let style = cell.style();
                if style.fg == Some(INACTIVE) || style.add_modifier.contains(Modifier::DIM) {
                    found_dimmed = true;
                }
            }
        }
        found_non_space && found_dimmed
    }

    fn render_board_lines(app: &App, area: Rect) -> Vec<String> {
        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_board(frame, app, area))
            .unwrap();

        let buffer = terminal.backend().buffer();
        (0..area.height)
            .map(|y| {
                let mut line = String::new();
                for x in 0..area.width {
                    let symbol = buffer
                        .cell((area.x + x, area.y + y))
                        .map(|cell| cell.symbol())
                        .unwrap_or(" ");
                    line.push_str(symbol);
                }
                line
            })
            .collect()
    }

    fn render_projects_lines(app: &App, area: Rect) -> Vec<String> {
        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_projects(frame, app, area))
            .unwrap();

        let buffer = terminal.backend().buffer();
        (0..area.height)
            .map(|y| {
                let mut line = String::new();
                for x in 0..area.width {
                    let symbol = buffer
                        .cell((area.x + x, area.y + y))
                        .map(|cell| cell.symbol())
                        .unwrap_or(" ");
                    line.push_str(symbol);
                }
                line
            })
            .collect()
    }

    fn board_render_includes_board_content(content: &str) {
        assert!(content.contains("TODO"));
        assert!(
            content.contains("first task") || content.contains("second task"),
            "expected a card title to be rendered"
        );
    }

    fn board_render_empty_list_shows_no_cards_and_hint(content: &str) {
        assert!(
            content.contains("No cards"),
            "expected No cards placeholder"
        );
        assert!(
            content.contains("Press n to create a card"),
            "expected placeholder hint"
        );
    }

    fn board_renders_preview(app: &App, area: Rect, show_preview: bool) -> String {
        let lines = render_board_lines(app, area);
        let content = lines.join("\n");
        if show_preview {
            assert!(content.contains(" Preview "));
        } else {
            assert!(!content.contains(" Preview "));
        }
        board_render_includes_board_content(&content);
        content
    }

    #[test]
    fn board_details_text_shows_board_summary_and_lists() {
        let board = Board {
            name: "Work Board".to_string(),
            path: PathBuf::from("work"),
            lists: vec![
                List {
                    name: "Todo".to_string(),
                    path: PathBuf::from("work/todo"),
                    cards: vec![card("Card A", "a.md"), card("Card B", "b.md")],
                },
                List {
                    name: "Done".to_string(),
                    path: PathBuf::from("work/done"),
                    cards: Vec::new(),
                },
            ],
        };

        assert_eq!(
            board_details_plain_text(&board),
            "Work Board\n2 lists / 2 cards\n\nTodo  2 cards\n  - Card A\n  - Card B\n\nDone  0 cards\n  No cards"
        );
    }

    #[test]
    fn render_projects_keeps_wide_board_details_layout() {
        let app = sample_projects_app(Mode::Normal);
        let lines = render_projects_lines(&app, Rect::new(0, 0, 80, 12));
        let board_details_title_x = lines
            .iter()
            .find_map(|line| {
                line.find(" Board Details ")
                    .map(|index| line[..index].chars().count())
            })
            .expect("board details title should render");

        assert_eq!(board_details_title_x, 31);
    }

    #[test]
    fn render_full_projects_keeps_wide_board_details_layout() {
        let app = sample_projects_app(Mode::Normal);
        let lines = render_tui_lines(&app, Rect::new(0, 0, 80, 14));
        let board_details_title_x = lines
            .iter()
            .find_map(|line| {
                line.find(" Board Details ")
                    .map(|index| line[..index].chars().count())
            })
            .expect("board details title should render");

        assert_eq!(board_details_title_x, 32);
    }

    #[test]
    fn rename_popup_text_shows_target_name_and_current_input() {
        assert_eq!(
            rename_popup_text(RenameTarget::Card, "Ship preview"),
            "Rename Card\n\nShip preview\n\nEnter Rename  Esc Cancel"
        );
    }

    #[test]
    fn board_header_text_is_compact() {
        let board = Board {
            name: "strike".to_string(),
            path: PathBuf::from("strike"),
            lists: vec![
                List {
                    name: "TODO".to_string(),
                    path: PathBuf::from("todo"),
                    cards: vec![card("task", "task.md")],
                },
                List {
                    name: "DONE".to_string(),
                    path: PathBuf::from("done"),
                    cards: Vec::new(),
                },
            ],
        };

        assert_eq!(
            board_header_text(&board, Mode::Normal.name()),
            "✦ strike   ·  2 lists   ·  1 card      NORMAL"
        );
    }

    #[test]
    fn board_header_text_handles_singular_counts() {
        let board = Board {
            name: "solo".to_string(),
            path: PathBuf::from("solo"),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("todo"),
                cards: vec![card("task", "task.md")],
            }],
        };

        assert_eq!(
            board_header_text(&board, Mode::Normal.name()),
            "✦ solo   ·  1 list   ·  1 card      NORMAL"
        );
    }

    #[test]
    fn list_title_includes_card_count() {
        assert_eq!(list_title("TODO", 1, false), " TODO [1] ");
        assert_eq!(list_title("DONE", 0, false), " DONE [0] ");
        assert_eq!(list_title("PENDING", 3, true), " -> PENDING [3] ");
    }

    #[test]
    fn palette_uses_warm_niffler_colors() {
        assert_eq!(ACTIVE_SELECTION, Color::Rgb(218, 173, 82));
        assert_eq!(HEADER, Color::Rgb(236, 196, 91));
        assert_eq!(PANEL, Color::Rgb(151, 111, 58));
        assert_ne!(ACTIVE_SELECTION, Color::LightCyan);
        assert_ne!(SHELL, Color::Blue);
    }

    #[test]
    fn empty_list_rows_show_action_hint() {
        assert_eq!(
            empty_list_rows(),
            vec![
                BoardRow::Placeholder("No cards"),
                BoardRow::Placeholder("Press n to create a card"),
            ]
        );
    }

    #[test]
    fn board_row_selectability_distinguishes_placeholder_and_card_rows() {
        assert!(!BoardRow::Placeholder("No cards").is_selectable());
        assert!(BoardRow::Card("task").is_selectable());
    }

    #[test]
    fn list_block_style_marks_selected_and_move_targets() {
        assert_eq!(
            list_block_style(true, false, false),
            Style::default().fg(HEADER).add_modifier(Modifier::BOLD),
        );
        assert_eq!(
            list_block_style(false, true, false),
            Style::default()
                .fg(MOVE_TARGET)
                .add_modifier(Modifier::BOLD),
        );
        assert_eq!(
            list_block_style(false, false, false),
            Style::default().fg(UNFOCUSED_PANEL_BORDER)
        );
    }

    #[test]
    fn card_row_style_for_placeholder_is_dimmed_and_non_selected_cards_get_list_colors() {
        let style = card_row_style(&BoardRow::Placeholder("No cards"), false, true, false);
        assert_eq!(style, Style::default().fg(INACTIVE));

        assert_eq!(
            card_row_style(&BoardRow::Card("card"), false, true, false),
            Style::default().fg(TEXT)
        );
        assert_eq!(
            card_row_style(&BoardRow::Card("card"), false, false, false),
            Style::default().fg(MUTED)
        );
    }

    #[test]
    fn selected_card_row_is_highlighted_with_bold_background() {
        let selected_style = card_row_style(&BoardRow::Card("card"), true, true, false);
        assert_eq!(
            selected_style,
            Style::default()
                .fg(SELECTED_TEXT)
                .bg(ACTIVE_SELECTION)
                .add_modifier(Modifier::BOLD),
        );

        let selected_move_style = card_row_style(&BoardRow::Card("card"), true, true, true);
        assert_eq!(
            selected_move_style,
            Style::default()
                .fg(SELECTED_TEXT)
                .bg(MOVE_TARGET)
                .add_modifier(Modifier::BOLD),
        );
    }

    #[test]
    fn move_list_mode_does_not_select_board_rows() {
        assert!(!is_row_selected(
            &Mode::MoveList { target_position: 1 },
            &BoardRow::Card("task"),
            true,
            0,
            3
        ));
        assert!(!is_row_selected(
            &Mode::MoveList { target_position: 2 },
            &BoardRow::MoveMarker("moving card".to_string()),
            true,
            0,
            3
        ));
    }

    #[test]
    fn move_list_mode_keeps_visible_anchor_at_top_with_deep_selected_card() {
        let board = Board {
            name: "focus".to_string(),
            path: PathBuf::from("focus"),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("todo"),
                cards: (0u8..10)
                    .map(|index| card(&format!("card {index:02}"), &format!("card-{index}.md")))
                    .collect(),
            }],
        };
        let app = App {
            root: PathBuf::from("/tmp"),
            screen: Screen::Board,
            mode: Mode::MoveList { target_position: 0 },
            show_preview: false,
            projects: Vec::new(),
            board: Some(board),
            project_preview: None,
            selected_project: 0,
            selected_list: 0,
            selected_cards: vec![9],
            status: String::new(),
            should_quit: false,
        };

        let content = render_board_lines(&app, Rect::new(0, 0, 80, 10)).join("\n");
        assert!(content.contains("card 00"));
        assert!(!content.contains("card 08"));
    }

    #[test]
    fn render_empty_list_in_normal_mode_shows_placeholders() {
        let board = Board {
            name: "strike".to_string(),
            path: PathBuf::from("strike"),
            lists: vec![List {
                name: "TODO".to_string(),
                path: PathBuf::from("todo"),
                cards: Vec::new(),
            }],
        };
        let app = App {
            root: PathBuf::from("/tmp"),
            screen: Screen::Board,
            mode: Mode::Normal,
            show_preview: false,
            projects: Vec::new(),
            board: Some(board),
            project_preview: None,
            selected_project: 0,
            selected_list: 0,
            selected_cards: vec![0],
            status: String::new(),
            should_quit: false,
        };

        let content = render_board_lines(&app, Rect::new(0, 0, 60, 14)).join("\n");
        board_render_empty_list_shows_no_cards_and_hint(&content);
    }

    #[test]
    fn render_empty_list_in_movelist_mode_shows_placeholders() {
        let board = Board {
            name: "strike".to_string(),
            path: PathBuf::from("strike"),
            lists: vec![
                List {
                    name: "TODO".to_string(),
                    path: PathBuf::from("todo"),
                    cards: vec![card("first task", "task1.md")],
                },
                List {
                    name: "DONE".to_string(),
                    path: PathBuf::from("done"),
                    cards: Vec::new(),
                },
            ],
        };
        let app = App {
            root: PathBuf::from("/tmp"),
            screen: Screen::Board,
            mode: Mode::MoveList { target_position: 1 },
            show_preview: false,
            projects: Vec::new(),
            board: Some(board),
            project_preview: None,
            selected_project: 0,
            selected_list: 0,
            selected_cards: vec![0, 0],
            status: String::new(),
            should_quit: false,
        };

        let content = render_board_lines(&app, Rect::new(0, 0, 80, 14)).join("\n");
        board_render_empty_list_shows_no_cards_and_hint(&content);
    }

    #[test]
    fn render_empty_target_list_in_move_mode_renders_move_marker_not_placeholders() {
        let board = Board {
            name: "strike".to_string(),
            path: PathBuf::from("strike"),
            lists: vec![
                List {
                    name: "TODO".to_string(),
                    path: PathBuf::from("todo"),
                    cards: vec![card("first task", "task1.md")],
                },
                List {
                    name: "DONE".to_string(),
                    path: PathBuf::from("done"),
                    cards: Vec::new(),
                },
            ],
        };
        let app = App {
            root: PathBuf::from("/tmp"),
            screen: Screen::Board,
            mode: Mode::Move {
                target_list: 1,
                target_position: 0,
            },
            show_preview: false,
            projects: Vec::new(),
            board: Some(board),
            project_preview: None,
            selected_project: 0,
            selected_list: 0,
            selected_cards: vec![0, 0],
            status: String::new(),
            should_quit: false,
        };

        let content = render_board_lines(&app, Rect::new(0, 0, 80, 14)).join("\n");
        assert!(content.contains("-> first task"));
        assert!(!content.contains("No cards"));
        assert!(!content.contains("Press n to create a card"));
    }

    #[test]
    fn board_content_layout_without_preview_uses_full_area() {
        let area = Rect::new(2, 4, 80, 20);
        assert_eq!(board_content_layout(area, false), (area, None));
    }

    #[test]
    fn board_content_layout_with_preview_splits_area_horizontally() {
        let area = Rect::new(3, 5, 80, 20);
        let (lists_area, preview_area) = board_content_layout(area, true);

        let preview_area = preview_area.expect("preview should be shown");
        assert_eq!(lists_area.x, area.x);
        assert_eq!(lists_area.y, area.y);
        assert_eq!(preview_area.y, area.y);
        assert_eq!(lists_area.height, area.height);
        assert_eq!(preview_area.height, area.height);
        assert_eq!(preview_area.x, lists_area.x + lists_area.width);
        assert_eq!(lists_area.width, 40);
        assert_eq!(preview_area.width, 40);
        assert_eq!(lists_area.width + preview_area.width, area.width);
    }

    #[test]
    fn board_content_layout_small_widths_with_preview_are_safe() {
        for width in 0u16..=4 {
            let area = Rect::new(7, 9, width, 12);
            let (lists_area, preview_area) = board_content_layout(area, true);

            assert_eq!(lists_area.x, area.x);
            assert_eq!(lists_area.y, area.y);
            assert_eq!(lists_area.height, area.height);
            assert!(lists_area.width <= area.width);

            match width {
                0..=3 => assert_eq!(preview_area, None),
                4 => {
                    let preview_area =
                        preview_area.expect("width 4 should still allocate a preview slot");
                    assert_eq!(preview_area.x, lists_area.x + lists_area.width);
                    assert_eq!(preview_area.y, area.y);
                    assert_eq!(preview_area.height, area.height);
                    assert_eq!(lists_area.width + preview_area.width, area.width);
                }
                _ => unreachable!("test only iterates widths 0 through 4"),
            }
        }
    }

    #[test]
    fn visible_list_window_keeps_focused_list_visible() {
        assert_eq!(visible_list_window(0, 3, 6), 0..3);
        assert_eq!(visible_list_window(3, 3, 6), 1..4);
        assert_eq!(visible_list_window(5, 3, 6), 3..6);
    }

    #[test]
    fn visible_list_window_handles_small_or_empty_inputs() {
        assert_eq!(visible_list_window(2, 10, 4), 0..4);
        assert_eq!(visible_list_window(9, 3, 6), 3..6);
        assert_eq!(visible_list_window(0, 0, 6), 0..0);
        assert_eq!(visible_list_window(0, 3, 0), 0..0);
    }

    #[test]
    fn render_board_with_preview_renders_preview_title() {
        let app = sample_board_app(true);
        board_renders_preview(&app, Rect::new(0, 0, 80, 16), true);
    }

    #[test]
    fn render_board_with_preview_places_preview_on_right_half() {
        let app = sample_board_app(true);
        let lines = render_board_lines(&app, Rect::new(0, 0, 80, 16));
        let preview_title_x = lines
            .iter()
            .find_map(|line| {
                line.find(" Preview ")
                    .map(|index| line[..index].chars().count())
            })
            .expect("preview title should render");

        assert_eq!(preview_title_x, 41);
    }

    #[test]
    fn render_full_board_with_preview_places_preview_on_right_half() {
        let app = sample_board_app(true);
        let lines = render_tui_lines(&app, Rect::new(0, 0, 80, 18));
        let preview_title_x = lines
            .iter()
            .find_map(|line| {
                line.find(" Preview ")
                    .map(|index| line[..index].chars().count())
            })
            .expect("preview title should render");

        assert_eq!(preview_title_x, 41);
    }

    #[test]
    fn render_wide_board_with_preview_uses_half_screen() {
        let app = sample_board_app(true);
        let width = 186;
        let lines = render_tui_lines(&app, Rect::new(0, 0, width, 30));
        let preview_title_x = lines
            .iter()
            .find_map(|line| {
                line.find(" Preview ")
                    .map(|index| line[..index].chars().count())
            })
            .expect("preview title should render");

        assert_eq!(preview_title_x, width as usize / 2 + 1);
    }

    #[test]
    fn render_board_in_narrow_area_still_renders_content() {
        let app = sample_board_app(false);
        let lines = render_board_lines(&app, Rect::new(0, 0, 20, 4));
        let content = lines.join("\n");
        board_render_includes_board_content(&content);
        assert!(!content.contains(" Preview "));
    }

    #[test]
    fn render_board_no_preview_renders_without_preview_title() {
        let app = sample_board_app(false);
        board_renders_preview(&app, Rect::new(0, 0, 60, 16), false);
    }

    #[test]
    fn has_modal_matches_project_modal_modes() {
        assert!(has_modal(&Mode::CreateProject {
            input: "Board".to_string()
        }));
        assert!(has_modal(&Mode::CreateList {
            input: "List".to_string()
        }));
        assert!(has_modal(&Mode::Add {
            input: "Card".to_string()
        }));
        assert!(has_modal(&Mode::Rename {
            target: RenameTarget::Card,
            input: "Rename".to_string()
        }));
        assert!(has_modal(&Mode::ConfirmDelete {
            target: DeleteTarget::Card
        }));
        assert!(has_modal(&Mode::Help));
    }

    #[test]
    fn has_modal_rejects_non_modal_modes() {
        assert!(!has_modal(&Mode::Normal));
        assert!(!has_modal(&Mode::Move {
            target_list: 0,
            target_position: 0
        }));
        assert!(!has_modal(&Mode::MoveList { target_position: 0 }));
    }

    #[test]
    fn home_footer_text_is_compact_and_accurate() {
        let app = sample_projects_app(Mode::Normal);
        assert_eq!(
            instruction_line(&app),
            "[NORMAL] [Tab] Switch [j/k] Move [Enter] Open [n] New [r] Rename [?] Help [q] Quit"
        );
    }

    #[test]
    fn board_footer_text_is_compact_and_accurate() {
        let app = sample_board_app(false);
        assert_eq!(
            instruction_line(&app),
            "[NORMAL] [Tab] List [j/k] Move [n] New [m] Move [p] Preview [?] Help [q] Quit"
        );
        let app = sample_board_app(true);
        assert_eq!(
            instruction_line(&app),
            "[NORMAL] [Tab] List [j/k] Move [n] New [m] Move [p] Hide Preview [?] Help [q] Quit"
        );
    }

    #[test]
    fn board_help_text_includes_preview_toggle() {
        let text = help_text(Screen::Board);
        assert!(text.contains("Navigation"));
        assert!(text.contains("Cards"));
        assert!(text.contains("Lists"));
        assert!(text.contains("Help"));
        assert!(text.contains("Tab/h/l"));
        assert!(text.contains("n New Card"));
        assert!(text.contains("N New List"));
        assert!(text.contains("p Preview"));
        assert!(text.contains(HELP_CLOSE_LINE));
        assert!(text.contains("? / Esc / q"));
        assert!(!text.contains("q Quit"));
    }

    #[test]
    fn home_help_text_includes_board_actions() {
        let text = help_text(Screen::Projects);
        assert!(text.contains("Navigation"));
        assert!(text.contains("Boards"));
        assert!(text.contains("j/k Up/Down"));
        assert!(text.contains("Enter Open"));
        assert!(text.contains("n New Board"));
        assert!(text.contains("r Rename"));
        assert!(text.contains(HELP_CLOSE_LINE));
        assert!(text.contains("? / Esc / q"));
        assert!(!text.contains("q Quit"));
    }

    #[test]
    fn help_modal_renders_and_dims_background_at_constrained_size() {
        let app = sample_projects_app(Mode::Help);
        let area = Rect::new(0, 0, 40, 12);
        let buffer = render_tui_buffer(&app, area);
        let lines = render_tui_lines(&app, area).join("\n");
        let popup = help_popup_area(area, help_text(Screen::Projects).as_str());
        let popup_lines = render_tui_lines_in_rect(&buffer, popup).join("\n");

        assert!(lines.contains("Keyboard"));
        assert!(lines.contains("Shortcuts"));
        assert!(popup_lines.contains("? / Esc / q"));
        assert!(popup_lines.contains("Close help"));
        assert!(has_dimmed_background_visible(&buffer, area, popup));
    }

    #[test]
    fn help_modal_wraps_text_on_narrow_board_view() {
        let mut app = sample_board_app(false);
        app.mode = Mode::Help;
        let area = Rect::new(0, 0, 22, 24);
        let buffer = render_tui_buffer(&app, area);
        let lines = render_tui_lines(&app, area).join("\n");
        let popup = help_popup_area(area, help_text(Screen::Board).as_str());
        let popup_lines = render_tui_lines_in_rect(&buffer, popup).join("\n");

        assert!(lines.contains("Keyboard"));
        assert!(lines.contains("Shortcuts"));
        assert!(popup_lines.contains("? / Esc / q"));
        assert!(popup_lines.contains("Close help") || popup_lines.contains("Close"));
        assert!(lines.contains("Tab/h/l"));
        assert!(lines.contains("List"));
        assert!(has_dimmed_background_visible(&buffer, area, popup));
    }

    #[test]
    fn wrapped_text_lines_counts_wrapping() {
        assert_eq!(wrapped_text_lines("a very long line of words", 10), 3);
        assert_eq!(wrapped_text_lines("singlewordthatissolong", 6), 4);
        assert_eq!(wrapped_text_lines("short\nline", 10), 2);
    }

    #[test]
    fn create_project_modal_renders_content_and_keeps_background_visible_and_dimmed() {
        let app = sample_projects_app(Mode::CreateProject {
            input: "Roadmap".to_string(),
        });
        let area = Rect::new(0, 0, 80, 24);
        let buffer = render_tui_buffer(&app, area);
        let lines = render_tui_lines(&app, area).join("\n");
        let popup = centered_rect(64, 40, area);

        assert!(lines.contains("New Board"));
        assert!(lines.contains("Board name"));
        assert!(lines.contains("Roadmap"));
        assert!(lines.contains("Enter Create  Esc Cancel"));
        assert!(has_dimmed_background_visible(&buffer, area, popup));
    }

    #[test]
    fn rename_modal_renders_content_and_keeps_background_visible_and_dimmed() {
        let app = sample_projects_app(Mode::Rename {
            target: RenameTarget::Board,
            input: "Renamed Board".to_string(),
        });
        let area = Rect::new(0, 0, 80, 24);
        let buffer = render_tui_buffer(&app, area);
        let lines = render_tui_lines(&app, area).join("\n");
        let popup = centered_rect(64, 40, area);

        assert!(lines.contains("Rename"));
        assert!(lines.contains("Renamed Board"));
        assert!(lines.contains("Enter Rename  Esc Cancel"));
        assert!(has_dimmed_background_visible(&buffer, area, popup));
    }

    #[test]
    fn input_popup_text_shows_label_input_and_action() {
        assert_eq!(
            input_popup_text("Card title", "Ship preview", "Create"),
            "Card title\n\nShip preview\n\nEnter Create  Esc Cancel"
        );
    }

    fn card(title: &str, filename: &str) -> Card {
        Card {
            title: title.to_string(),
            filename: filename.to_string(),
            path: PathBuf::from(filename),
            content: String::new(),
            position: 0,
        }
    }
}
