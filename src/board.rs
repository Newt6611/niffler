use crate::card::Card;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorOption {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeColors {
    pub active_selection: String,
    pub header: String,
    pub success: String,
    pub inactive: String,
    pub unfocused_panel_border: String,
    pub text: String,
    pub muted: String,
    pub selected_text: String,
    pub shell: String,
    pub panel: String,
    pub preview: String,
    pub modal: String,
    pub move_target: String,
    pub danger: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub theme: ThemeColors,
    pub colors: Vec<ColorOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct List {
    pub name: String,
    pub path: PathBuf,
    pub cards: Vec<Card>,
    pub border_color: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board {
    pub name: String,
    pub path: PathBuf,
    pub theme: ThemeColors,
    pub colors: Vec<ColorOption>,
    pub lists: Vec<List>,
}

pub fn default_color_options() -> Vec<ColorOption> {
    vec![
        color_option("Default", "#3c3c3c"),
        color_option("Amber", "#f59e0b"),
        color_option("Green", "#22c55e"),
        color_option("Sky", "#38bdf8"),
        color_option("Violet", "#a855f7"),
        color_option("Rose", "#f43f5e"),
        color_option("Teal", "#14b8a6"),
        color_option("Slate", "#64748b"),
    ]
}

pub fn default_app_config() -> AppConfig {
    AppConfig {
        theme: default_theme_colors(),
        colors: default_color_options(),
    }
}

pub fn default_theme_colors() -> ThemeColors {
    ThemeColors {
        active_selection: "#daad52".to_string(),
        header: "#ecc45b".to_string(),
        success: "#be8f42".to_string(),
        inactive: "#58544c".to_string(),
        unfocused_panel_border: "#3c3c3c".to_string(),
        text: "#e5dbc7".to_string(),
        muted: "#9a8e7a".to_string(),
        selected_text: "black".to_string(),
        shell: "#7a582a".to_string(),
        panel: "#976f3a".to_string(),
        preview: "#b0803e".to_string(),
        modal: "#f4cf68".to_string(),
        move_target: "#d39743".to_string(),
        danger: "#c65d4a".to_string(),
    }
}

fn color_option(label: &str, value: &str) -> ColorOption {
    ColorOption {
        label: label.to_string(),
        value: value.to_string(),
    }
}
