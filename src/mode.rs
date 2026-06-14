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
        }
    }
}
