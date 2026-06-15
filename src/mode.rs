#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Projects,
    Board,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Normal,
    CreateProject {
        input: String,
    },
    CreateList {
        input: String,
    },
    Add {
        input: String,
    },
    Rename {
        target: RenameTarget,
        input: String,
    },
    Help,
    Move {
        target_list: usize,
        target_position: usize,
    },
    MoveList {
        target_position: usize,
    },
    ConfirmDelete {
        target: DeleteTarget,
    },
    Picker(PickerState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeleteTarget {
    Board,
    List,
    Card,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenameTarget {
    Board,
    List,
    Card,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PickerState {
    pub title: String,
    pub options: Vec<PickerOption>,
    pub selected: usize,
    pub target: PickerTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PickerOption {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickerTarget {
    ListBorderColor {
        list_index: usize,
    },
    CardColor {
        list_index: usize,
        card_index: usize,
    },
}

impl Mode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::CreateProject { .. } => "New Board",
            Self::CreateList { .. } => "New List",
            Self::Add { .. } => "Add",
            Self::Rename { .. } => "Rename",
            Self::Help => "Help",
            Self::Move { .. } => "Move",
            Self::MoveList { .. } => "Move List",
            Self::ConfirmDelete { .. } => "Confirm",
            Self::Picker(_) => "Picker",
        }
    }
}
