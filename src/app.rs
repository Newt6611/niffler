use crate::board::{AppConfig, Board, Project};
use crate::card::preview_content;
use crate::filesystem;
use crate::mode::{DeleteTarget, Mode, PickerOption, PickerState, PickerTarget, Screen};
use std::io;
use std::path::PathBuf;

pub struct App {
    pub root: PathBuf,
    pub config: AppConfig,
    pub screen: Screen,
    pub mode: Mode,
    pub show_preview: bool,
    pub projects: Vec<Project>,
    pub board: Option<Board>,
    pub project_preview: Option<Board>,
    pub selected_project: usize,
    pub selected_list: usize,
    pub selected_cards: Vec<usize>,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        let config = filesystem::load_config(&root)?;
        let projects = filesystem::load_projects(&root)?;
        let project_preview = projects
            .first()
            .and_then(|project| filesystem::load_board_with_config(&project.path, &config).ok());
        Ok(Self {
            root,
            config,
            screen: Screen::Projects,
            mode: Mode::Normal,
            show_preview: false,
            projects,
            board: None,
            project_preview,
            selected_project: 0,
            selected_list: 0,
            selected_cards: Vec::new(),
            status: String::new(),
            should_quit: false,
        })
    }

    pub fn refresh_projects(&mut self) -> io::Result<()> {
        self.projects = filesystem::load_projects(&self.root)?;
        clamp_index(&mut self.selected_project, self.projects.len());
        self.refresh_project_preview();
        Ok(())
    }

    pub fn toggle_preview(&mut self) -> io::Result<()> {
        self.show_preview = !self.show_preview;
        if let Some(board) = &self.board {
            filesystem::set_board_preview(&board.path, self.show_preview)?;
        }
        Ok(())
    }

    pub fn list_border_color_picker(&self) -> Option<PickerState> {
        let board = self.board.as_ref()?;
        let list = board.lists.get(self.selected_list)?;
        let selected = list
            .border_color
            .as_deref()
            .and_then(|color| {
                self.config
                    .colors
                    .iter()
                    .position(|option| option.value.eq_ignore_ascii_case(color))
            })
            .unwrap_or(0);
        Some(PickerState {
            title: "List Border Color".to_string(),
            options: self
                .config
                .colors
                .iter()
                .map(|option| PickerOption {
                    label: option.label.clone(),
                    value: option.value.clone(),
                })
                .collect(),
            selected,
            target: PickerTarget::ListBorderColor {
                list_index: self.selected_list,
            },
        })
    }

    pub fn card_color_picker(&self) -> Option<PickerState> {
        let board = self.board.as_ref()?;
        let list = board.lists.get(self.selected_list)?;
        let card_index = *self.selected_cards.get(self.selected_list).unwrap_or(&0);
        let card = list.cards.get(card_index)?;
        let selected = card
            .color
            .as_deref()
            .and_then(|color| {
                self.config
                    .colors
                    .iter()
                    .position(|option| option.value.eq_ignore_ascii_case(color))
            })
            .unwrap_or(0);
        Some(PickerState {
            title: "Card Color".to_string(),
            options: self
                .config
                .colors
                .iter()
                .map(|option| PickerOption {
                    label: option.label.clone(),
                    value: option.value.clone(),
                })
                .collect(),
            selected,
            target: PickerTarget::CardColor {
                list_index: self.selected_list,
                card_index,
            },
        })
    }

    pub fn set_selected_list_border_color(&mut self, border_color: &str) -> io::Result<()> {
        self.set_list_border_color(self.selected_list, border_color)
    }

    pub fn set_list_border_color(
        &mut self,
        list_index: usize,
        border_color: &str,
    ) -> io::Result<()> {
        let Some(board) = &self.board else {
            self.status = "No board selected".to_string();
            return Ok(());
        };
        let Some(list) = board.lists.get(list_index) else {
            self.status = "No list selected".to_string();
            return Ok(());
        };
        let board_path = board.path.clone();
        let list_path = list.path.clone();
        filesystem::set_list_border_color(&board_path, &list_path, Some(border_color))?;
        self.reload_board()?;
        self.status = "Updated list border color".to_string();
        Ok(())
    }

    pub fn set_card_color(
        &mut self,
        list_index: usize,
        card_index: usize,
        color: &str,
    ) -> io::Result<()> {
        let Some(board) = &self.board else {
            self.status = "No board selected".to_string();
            return Ok(());
        };
        let Some(card) = board
            .lists
            .get(list_index)
            .and_then(|list| list.cards.get(card_index))
        else {
            self.status = "No card selected".to_string();
            return Ok(());
        };
        let card_path = card.path.clone();
        filesystem::set_card_color(&card_path, color)?;
        self.reload_board()?;
        self.status = "Updated card color".to_string();
        Ok(())
    }

    pub fn open_selected_project(&mut self) -> io::Result<()> {
        let Some(project) = self.projects.get(self.selected_project) else {
            self.status = format!("No projects in {}", self.root.display());
            return Ok(());
        };

        let board = filesystem::load_board_with_config(&project.path, &self.config)?;
        self.show_preview = filesystem::board_preview_setting(&project.path)?;
        self.selected_list = 0;
        self.selected_cards = vec![0; board.lists.len()];
        self.board = Some(board);
        self.screen = Screen::Board;
        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn switch_tab(&mut self) -> io::Result<()> {
        self.mode = Mode::Normal;
        match self.screen {
            Screen::Projects => {
                if self.board.is_some() {
                    self.screen = Screen::Board;
                    Ok(())
                } else {
                    self.open_selected_project()
                }
            }
            Screen::Board => {
                self.screen = Screen::Projects;
                Ok(())
            }
        }
    }

    pub fn reload_board(&mut self) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let path = board.path.clone();
        let board = filesystem::load_board_with_config(&path, &self.config)?;
        self.ensure_selection_shape(board.lists.len());
        self.board = Some(board);
        self.clamp_board_selection();
        Ok(())
    }

    pub fn current_project_name(&self) -> &str {
        match self.screen {
            Screen::Projects => "Projects",
            Screen::Board => self
                .board
                .as_ref()
                .map(|board| board.name.as_str())
                .unwrap_or("Board"),
        }
    }

    pub fn selected_card_content(&self) -> Option<String> {
        let board = self.board.as_ref()?;
        let list = board.lists.get(self.selected_list)?;
        let card = list
            .cards
            .get(*self.selected_cards.get(self.selected_list).unwrap_or(&0))?;
        Some(preview_content(&card.content))
    }

    pub fn selected_card_path(&self) -> Option<PathBuf> {
        let board = self.board.as_ref()?;
        let list = board.lists.get(self.selected_list)?;
        let card = list
            .cards
            .get(*self.selected_cards.get(self.selected_list).unwrap_or(&0))?;
        Some(card.path.clone())
    }

    pub fn move_project_selection(&mut self, delta: isize) {
        move_index(&mut self.selected_project, self.projects.len(), delta);
        self.refresh_project_preview();
    }

    fn refresh_project_preview(&mut self) {
        self.project_preview = self
            .projects
            .get(self.selected_project)
            .and_then(|project| {
                filesystem::load_board_with_config(&project.path, &self.config).ok()
            });
    }

    pub fn move_card_selection(&mut self, delta: isize) {
        let Some(board) = &self.board else {
            return;
        };
        let Some(list) = board.lists.get(self.selected_list) else {
            return;
        };
        let selected = self.selected_cards.get_mut(self.selected_list);
        if let Some(selected) = selected {
            move_index(selected, list.cards.len(), delta);
        }
    }

    pub fn move_list_selection(&mut self, delta: isize) {
        let Some(board) = &self.board else {
            return;
        };
        let len = board.lists.len();
        if len == 0 {
            self.selected_list = 0;
            return;
        }
        self.selected_list = wrap_index(self.selected_list, len, delta);
        self.ensure_selection_shape(len);
    }

    pub fn focus_next_list(&mut self) {
        self.move_list_selection(1);
    }

    pub fn focus_previous_list(&mut self) {
        self.move_list_selection(-1);
    }

    pub fn create_project(&mut self, name: &str) -> io::Result<()> {
        let path = filesystem::create_project(&self.root, name)?;
        self.refresh_projects()?;
        if let Some(index) = self
            .projects
            .iter()
            .position(|project| project.path == path)
        {
            self.selected_project = index;
        }
        self.status = format!(
            "Created board {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        Ok(())
    }

    pub fn delete_selected_project(&mut self) -> io::Result<()> {
        let Some(project) = self.projects.get(self.selected_project) else {
            self.status = "No board selected".to_string();
            return Ok(());
        };
        let deleted_path = project.path.clone();
        let deleted_name = project.name.clone();
        filesystem::delete_project(&deleted_path)?;
        if self
            .board
            .as_ref()
            .is_some_and(|board| board.path == deleted_path)
        {
            self.board = None;
            self.screen = Screen::Projects;
            self.selected_list = 0;
            self.selected_cards.clear();
        }
        self.refresh_projects()?;
        self.status = format!("Deleted board {deleted_name}");
        Ok(())
    }

    pub fn rename_selected_project(&mut self, name: &str) -> io::Result<()> {
        let Some(project) = self.projects.get(self.selected_project) else {
            self.status = "No board selected".to_string();
            return Ok(());
        };
        let old_path = project.path.clone();
        let new_path = filesystem::rename_project(&old_path, name)?;
        if self
            .board
            .as_ref()
            .is_some_and(|board| board.path == old_path)
        {
            self.board = Some(filesystem::load_board_with_config(&new_path, &self.config)?);
        }
        self.refresh_projects()?;
        if let Some(index) = self
            .projects
            .iter()
            .position(|project| project.path == new_path)
        {
            self.selected_project = index;
        }
        self.status = format!("Renamed board {}", name.trim());
        Ok(())
    }

    pub fn delete_selected_list(&mut self) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let Some(list) = board.lists.get(self.selected_list) else {
            self.status = "No list selected".to_string();
            return Ok(());
        };
        let deleted_name = list.name.clone();
        let deleted_path = list.path.clone();
        filesystem::delete_list(&deleted_path)?;
        self.status = format!("Deleted list {deleted_name}");
        self.reload_board()
    }

    pub fn rename_selected_list(&mut self, name: &str) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let Some(list) = board.lists.get(self.selected_list) else {
            self.status = "No list selected".to_string();
            return Ok(());
        };
        let new_path = filesystem::rename_list(&list.path, name)?;
        self.status = format!("Renamed list {}", name.trim());
        self.reload_board()?;
        if let Some(board) = &self.board {
            if let Some(index) = board.lists.iter().position(|list| list.path == new_path) {
                self.selected_list = index;
                self.ensure_selection_shape(board.lists.len());
            }
        }
        Ok(())
    }

    pub fn confirm_delete(&mut self, target: DeleteTarget) -> io::Result<()> {
        match target {
            DeleteTarget::Board => self.delete_selected_project(),
            DeleteTarget::List => self.delete_selected_list(),
            DeleteTarget::Card => self.delete_selected_card(),
        }
    }

    pub fn create_list(&mut self, name: &str) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let path = filesystem::create_list(&board.path, name)?;
        self.reload_board()?;
        if let Some(board) = &self.board {
            if let Some(index) = board.lists.iter().position(|list| list.path == path) {
                self.selected_list = index;
                self.ensure_selection_shape(board.lists.len());
            }
        }
        self.status = format!(
            "Created list {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        Ok(())
    }

    pub fn create_card(&mut self, title: &str) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let Some(list) = board.lists.get(self.selected_list) else {
            return Ok(());
        };

        let path = filesystem::create_card(&list.path, title)?;
        self.status = format!(
            "Created {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.reload_board()
    }

    pub fn delete_selected_card(&mut self) -> io::Result<()> {
        let Some(path) = self.selected_card_path() else {
            self.status = "No card selected".to_string();
            return Ok(());
        };
        filesystem::delete_card(&path)?;
        self.status = format!(
            "Deleted {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.reload_board()
    }

    pub fn rename_selected_card(&mut self, title: &str) -> io::Result<()> {
        let Some(path) = self.selected_card_path() else {
            self.status = "No card selected".to_string();
            return Ok(());
        };
        let new_path = filesystem::rename_card(&path, title)?;
        self.status = format!("Renamed card {}", title.trim());
        self.reload_board()?;
        if let Some(board) = &self.board {
            if let Some(list) = board.lists.get(self.selected_list) {
                if let Some(card_index) = list.cards.iter().position(|card| card.path == new_path) {
                    if let Some(selected_card) = self.selected_cards.get_mut(self.selected_list) {
                        *selected_card = card_index;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn move_selected_card_to(
        &mut self,
        target_list: usize,
        target_position: usize,
    ) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let Some(target) = board.lists.get(target_list) else {
            return Ok(());
        };
        let Some(card_path) = self.selected_card_path() else {
            self.status = "No card selected".to_string();
            return Ok(());
        };

        let moved_path = filesystem::move_card_to_index(&card_path, &target.path, target_position)?;
        self.selected_list = target_list;
        self.status = "Moved card".to_string();
        self.reload_board()?;
        if let Some(board) = &self.board {
            if let Some(list) = board.lists.get(target_list) {
                if let Some(card_index) = list.cards.iter().position(|card| card.path == moved_path)
                {
                    if let Some(selected_card) = self.selected_cards.get_mut(target_list) {
                        *selected_card = card_index;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn move_selected_list_to(&mut self, target_position: usize) -> io::Result<()> {
        let Some(board) = &self.board else {
            return Ok(());
        };
        let Some(list) = board.lists.get(self.selected_list) else {
            self.status = "No list selected".to_string();
            return Ok(());
        };

        let board_path = board.path.clone();
        let list_path = list.path.clone();
        let list_name = list.name.clone();
        filesystem::move_list_to_index(&board_path, &list_path, target_position)?;
        self.reload_board()?;
        if let Some(board) = &self.board {
            if let Some(index) = board.lists.iter().position(|list| list.path == list_path) {
                self.selected_list = index;
                self.ensure_selection_shape(board.lists.len());
            }
        }
        self.status = format!("Moved list {list_name}");
        Ok(())
    }

    pub fn move_target_position(
        &self,
        target_list: usize,
        target_position: usize,
        delta: isize,
    ) -> usize {
        let Some(board) = &self.board else {
            return 0;
        };
        let Some(list) = board.lists.get(target_list) else {
            return 0;
        };
        let max = list.cards.len();
        (target_position as isize + delta).clamp(0, max as isize) as usize
    }

    pub fn default_move_target_position(&self) -> usize {
        self.selected_cards
            .get(self.selected_list)
            .copied()
            .unwrap_or(0)
    }

    pub fn move_list_target_position(&self, target_position: usize, delta: isize) -> usize {
        let Some(board) = &self.board else {
            return 0;
        };
        move_index_value(target_position, board.lists.len(), delta)
    }

    fn ensure_selection_shape(&mut self, len: usize) {
        if self.selected_cards.len() < len {
            self.selected_cards.resize(len, 0);
        } else {
            self.selected_cards.truncate(len);
        }
    }

    fn clamp_board_selection(&mut self) {
        let Some(board) = &self.board else {
            return;
        };
        let list_len = board.lists.len();
        let card_lens = board
            .lists
            .iter()
            .map(|list| list.cards.len())
            .collect::<Vec<_>>();
        clamp_index(&mut self.selected_list, list_len);
        self.ensure_selection_shape(list_len);
        for (index, card_len) in card_lens.iter().enumerate() {
            if let Some(selected) = self.selected_cards.get_mut(index) {
                clamp_index(selected, *card_len);
            }
        }
    }
}

fn clamp_index(index: &mut usize, len: usize) {
    if len == 0 {
        *index = 0;
    } else if *index >= len {
        *index = len - 1;
    }
}

fn move_index(index: &mut usize, len: usize, delta: isize) {
    *index = move_index_value(*index, len, delta);
}

fn move_index_value(index: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }

    (index as isize + delta).clamp(0, len as isize - 1) as usize
}

fn wrap_index(index: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }

    (index as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_root() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "niffler-app-test-{}-{id}-{counter}",
            std::process::id()
        ))
    }

    #[test]
    fn create_project_refreshes_projects_and_selects_new_board() {
        let root = temp_root();
        let mut app = App::new(root.clone()).unwrap();

        app.create_project("Work Board").unwrap();

        assert_eq!(app.projects.len(), 1);
        assert_eq!(app.projects[app.selected_project].name, "work-board");
        assert_eq!(app.project_preview.as_ref().unwrap().name, "Work Board");
        assert!(root.join("work-board").is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_project_selection_refreshes_board_preview() {
        let root = temp_root();
        fs::create_dir_all(root.join("alpha/todo")).unwrap();
        fs::create_dir_all(root.join("zeta/done")).unwrap();
        fs::write(
            root.join("alpha/.niffler.yaml"),
            "name: Alpha Board\n\nlists:\n  - id: todo\n    title: Todo\n    position: 1000\n",
        )
        .unwrap();
        fs::write(
            root.join("zeta/.niffler.yaml"),
            "name: Zeta Board\n\nlists:\n  - id: done\n    title: Done\n    position: 1000\n",
        )
        .unwrap();
        let mut app = App::new(root.clone()).unwrap();

        assert_eq!(app.project_preview.as_ref().unwrap().name, "Alpha Board");

        app.move_project_selection(1);

        assert_eq!(app.projects[app.selected_project].name, "zeta");
        assert_eq!(app.project_preview.as_ref().unwrap().name, "Zeta Board");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn selected_card_content_hides_frontmatter_metadata() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(
            root.join("work/todo/task.md"),
            "---\nposition: 1000\ncreated_at: 123\nupdated_at: 124\n---\n\n# Task\nBody\n",
        )
        .unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        assert_eq!(
            app.selected_card_content().as_deref(),
            Some("# Task\nBody\n")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_selected_project_removes_board_and_refreshes_projects() {
        let root = temp_root();
        fs::create_dir_all(root.join("alpha/todo")).unwrap();
        fs::create_dir_all(root.join("zeta/todo")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.selected_project = 0;

        app.delete_selected_project().unwrap();

        assert!(!root.join("alpha").exists());
        assert_eq!(app.projects.len(), 1);
        assert_eq!(app.projects[app.selected_project].name, "zeta");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_loaded_project_clears_board_state() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.switch_tab().unwrap();

        app.delete_selected_project().unwrap();

        assert!(app.board.is_none());
        assert_eq!(app.screen, Screen::Projects);
        assert!(!root.join("work").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_selected_list_removes_directory_and_refreshes_board() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::create_dir_all(root.join("work/done")).unwrap();
        fs::write(root.join("work/todo/task.md"), "# Task\n").unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = 1;

        app.delete_selected_list().unwrap();

        let board = app.board.as_ref().unwrap();
        assert!(!root.join("work/todo").exists());
        assert_eq!(board.lists.len(), 1);
        assert_eq!(board.lists[app.selected_list].name, "done");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_list_reloads_board_and_selects_new_list() {
        let root = temp_root();
        fs::create_dir_all(root.join("work")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        app.create_list("In Progress").unwrap();

        let board = app.board.as_ref().unwrap();
        assert_eq!(board.lists.len(), 1);
        assert_eq!(board.lists[app.selected_list].name, "In Progress");
        assert!(root.join("work/in-progress").is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn focus_next_list_wraps_from_last_to_first() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::create_dir_all(root.join("work/doing")).unwrap();
        fs::create_dir_all(root.join("work/done")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = 2;

        app.focus_next_list();

        assert_eq!(app.selected_list, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn focus_previous_list_wraps_from_first_to_last() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::create_dir_all(root.join("work/doing")).unwrap();
        fs::create_dir_all(root.join("work/done")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = 0;

        app.focus_previous_list();

        assert_eq!(app.selected_list, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_list_selection_wraps_forward_from_last_to_first() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::create_dir_all(root.join("work/doing")).unwrap();
        fs::create_dir_all(root.join("work/done")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = 2;

        app.move_list_selection(1);

        assert_eq!(app.selected_list, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_list_selection_wraps_backward_from_first_to_last() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::create_dir_all(root.join("work/doing")).unwrap();
        fs::create_dir_all(root.join("work/done")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = 0;

        app.move_list_selection(-1);

        assert_eq!(app.selected_list, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn switch_tab_toggles_between_projects_and_loaded_board() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        let mut app = App::new(root.clone()).unwrap();

        app.switch_tab().unwrap();
        assert_eq!(app.screen, Screen::Board);
        assert_eq!(app.board.as_ref().unwrap().name, "work");

        app.switch_tab().unwrap();
        assert_eq!(app.screen, Screen::Projects);
        assert!(app.board.is_some());

        app.switch_tab().unwrap();
        assert_eq!(app.screen, Screen::Board);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_selected_card_focuses_destination_and_moved_card() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/source")).unwrap();
        fs::create_dir_all(root.join("work/target")).unwrap();
        fs::write(root.join("work/source/task.md"), "# Task\n").unwrap();
        fs::write(root.join("work/target/aaa.md"), "# Existing\n").unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = 0;
        app.selected_cards = vec![0, 0];

        app.move_selected_card_to(1, 0).unwrap();

        let board = app.board.as_ref().unwrap();
        assert_eq!(board.lists[app.selected_list].name, "target");
        assert_eq!(app.selected_cards[app.selected_list], 0);
        assert_eq!(board.lists[app.selected_list].cards[0].filename, "task.md");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_selected_list_reorders_board_and_keeps_focus() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::create_dir_all(root.join("work/doing")).unwrap();
        fs::create_dir_all(root.join("work/done")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();
        app.selected_list = app
            .board
            .as_ref()
            .unwrap()
            .lists
            .iter()
            .position(|list| list.path == root.join("work/done"))
            .unwrap();

        app.move_selected_list_to(0).unwrap();

        let board = app.board.as_ref().unwrap();
        assert_eq!(
            board
                .lists
                .iter()
                .map(|list| list.path.file_name().unwrap().to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["done", "doing", "todo"]
        );
        assert_eq!(board.lists[app.selected_list].name, "done");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_target_position_moves_within_target_list_bounds() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(root.join("work/todo/a.md"), "# A\n").unwrap();
        fs::write(root.join("work/todo/b.md"), "# B\n").unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        assert_eq!(app.move_target_position(0, 0, -1), 0);
        assert_eq!(app.move_target_position(0, 0, 1), 1);
        assert_eq!(app.move_target_position(0, 2, 1), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preview_panel_is_visible_by_default_and_can_toggle() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        assert!(app.show_preview);

        app.toggle_preview().unwrap();
        assert!(!app.show_preview);
        app.toggle_preview().unwrap();
        assert!(app.show_preview);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn opening_project_loads_preview_status_from_metadata() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(
            root.join("work/.niffler.yaml"),
            "name: Work\nshow_preview: true\n\nlists:\n  - id: todo\n    title: TODO\n    position: 1000\n",
        )
        .unwrap();
        let mut app = App::new(root.clone()).unwrap();

        app.open_selected_project().unwrap();

        assert!(app.show_preview);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn toggling_preview_persists_status_to_board_metadata() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(
            root.join("work/.niffler.yaml"),
            "name: Work\nshow_preview: false\n\nlists:\n  - id: todo\n    title: TODO\n    position: 1000\n",
        )
        .unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        app.toggle_preview().unwrap();

        assert!(app.show_preview);
        let metadata = fs::read_to_string(root.join("work/.niffler.yaml")).unwrap();
        assert!(metadata.contains("show_preview: true"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn selected_list_border_color_is_persisted_and_reloaded() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        app.set_selected_list_border_color("#38bdf8").unwrap();

        let board = app.board.as_ref().unwrap();
        assert_eq!(
            board.lists[app.selected_list].border_color.as_deref(),
            Some("#38bdf8")
        );
        let metadata = fs::read_to_string(root.join("work/.niffler.yaml")).unwrap();
        assert!(metadata.contains("border_color: \"#38bdf8\""));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn list_border_color_picker_uses_board_configured_colors() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(
            root.join("config.yaml"),
            "theme:\n  active_selection: \"#daad52\"\n\ncolors:\n  - label: Red\n    value: \"#ef4444\"\n  - label: Blue\n    value: \"#3b82f6\"\n",
        )
        .unwrap();
        fs::write(
            root.join("work/.niffler.yaml"),
            "name: Work\nshow_preview: false\n\nlists:\n  - id: todo\n    title: TODO\n    position: 1000\n    border_color: \"#3b82f6\"\n",
        )
        .unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        let picker = app.list_border_color_picker().unwrap();

        assert_eq!(picker.options[0].label, "Red");
        assert_eq!(picker.options[0].value, "#ef4444");
        assert_eq!(picker.options[1].label, "Blue");
        assert_eq!(picker.options[1].value, "#3b82f6");
        assert_eq!(picker.selected, 1);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn selected_card_color_is_persisted_and_reloaded() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(root.join("work/todo/task.md"), "# Task\n").unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        app.set_card_color(0, 0, "#38bdf8").unwrap();

        let board = app.board.as_ref().unwrap();
        assert_eq!(
            board.lists[app.selected_list].cards[0].color.as_deref(),
            Some("#38bdf8")
        );
        let content = fs::read_to_string(root.join("work/todo/task.md")).unwrap();
        assert!(content.contains("color: \"#38bdf8\""));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn card_color_picker_uses_board_configured_colors() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(
            root.join("config.yaml"),
            "theme:\n  active_selection: \"#daad52\"\n\ncolors:\n  - label: Red\n    value: \"#ef4444\"\n  - label: Blue\n    value: \"#3b82f6\"\n",
        )
        .unwrap();
        fs::write(
            root.join("work/todo/task.md"),
            "---\ncolor: \"#3b82f6\"\n---\n\n# Task\n",
        )
        .unwrap();
        let mut app = App::new(root.clone()).unwrap();
        app.open_selected_project().unwrap();

        let picker = app.card_color_picker().unwrap();

        assert_eq!(picker.title, "Card Color");
        assert_eq!(picker.options[0].label, "Red");
        assert_eq!(picker.options[0].value, "#ef4444");
        assert_eq!(picker.options[1].label, "Blue");
        assert_eq!(picker.options[1].value, "#3b82f6");
        assert_eq!(picker.selected, 1);
        assert_eq!(
            picker.target,
            PickerTarget::CardColor {
                list_index: 0,
                card_index: 0,
            }
        );
        fs::remove_dir_all(root).ok();
    }
}
