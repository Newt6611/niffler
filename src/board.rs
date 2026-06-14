use crate::card::Card;
use std::path::PathBuf;

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board {
    pub name: String,
    pub path: PathBuf,
    pub lists: Vec<List>,
}
