use crate::board::{Board, List, Project};
use crate::card::{
    Card, markdown_with_missing_metadata, markdown_with_position, markdown_with_title,
    markdown_with_updated_at, slug_from_title,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const BOARD_METADATA_FILE: &str = ".niffler.yaml";
const POSITION_STEP: i64 = 1000;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BoardMetadata {
    name: String,
    show_preview: bool,
    lists: Vec<ListMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListMetadata {
    id: String,
    title: String,
    position: i64,
}

pub fn load_projects(root: &Path) -> io::Result<Vec<Project>> {
    fs::create_dir_all(root)?;
    let mut projects = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            projects.push(Project {
                name: entry.file_name().to_string_lossy().to_string(),
                path,
            });
        }
    }

    projects.sort_by_key(|project| project.name.to_lowercase());
    Ok(projects)
}

pub fn load_board(path: &Path) -> io::Result<Board> {
    sync_board_metadata(path)?;
    let metadata = read_board_metadata(path)?;
    let mut lists = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let list_path = entry.path();
        if !list_path.is_dir() {
            continue;
        }

        let mut card_paths = markdown_files_in(&list_path)?;
        card_paths.sort_by_key(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });

        let mut cards = Vec::new();
        for (card_index, card_path) in card_paths.into_iter().enumerate() {
            let content = fs::read_to_string(&card_path)?;
            let position = ((card_index + 1) as i64) * POSITION_STEP;
            let content_with_metadata = markdown_with_missing_metadata(&content, position);
            if content_with_metadata != content {
                fs::write(&card_path, &content_with_metadata)?;
            }
            let filename = card_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            cards.push(Card::from_markdown(
                filename,
                card_path,
                content_with_metadata,
            ));
        }
        cards.sort_by(|left, right| {
            left.position.cmp(&right.position).then_with(|| {
                left.filename
                    .to_lowercase()
                    .cmp(&right.filename.to_lowercase())
            })
        });

        let id = entry.file_name().to_string_lossy().to_string();
        let title = metadata
            .as_ref()
            .and_then(|metadata| metadata.lists.iter().find(|list| list.id == id))
            .map(|list| list.title.clone())
            .unwrap_or_else(|| id.clone());

        lists.push(List {
            name: title,
            path: list_path,
            cards,
        });
    }

    lists.sort_by(|left, right| {
        list_sort_position(metadata.as_ref(), left)
            .cmp(&list_sort_position(metadata.as_ref(), right))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(Board {
        name: metadata
            .map(|metadata| metadata.name)
            .or_else(|| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "Board".to_string()),
        path: path.to_path_buf(),
        lists,
    })
}

pub fn create_project(root: &Path, name: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(root)?;
    let slug = slug_from_title(name);
    let mut path = root.join(&slug);
    let mut suffix = 2;
    while path.exists() {
        path = root.join(format!("{slug}-{suffix}"));
        suffix += 1;
    }

    fs::create_dir(&path)?;
    write_board_metadata(
        &path,
        &BoardMetadata {
            name: name.trim().to_string(),
            show_preview: false,
            lists: Vec::new(),
        },
    )?;
    Ok(path)
}

pub fn delete_project(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path)
}

pub fn rename_project(path: &Path, name: &str) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "project path has no parent"))?;
    let target = unique_path_except(parent, &slug_from_title(name), None, path);
    if target != path {
        fs::rename(path, &target)?;
    }
    sync_board_metadata(&target)?;
    if let Some(mut metadata) = read_board_metadata(&target)? {
        metadata.name = name.trim().to_string();
        write_board_metadata(&target, &metadata)?;
    }
    Ok(target)
}

pub fn delete_list(path: &Path) -> io::Result<()> {
    let board_path = path.parent().map(Path::to_path_buf);
    fs::remove_dir_all(path)?;
    if let Some(board_path) = board_path {
        sync_board_metadata(&board_path)?;
    }
    Ok(())
}

pub fn create_list(board_path: &Path, name: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(board_path)?;
    let slug = slug_from_title(name);
    let mut path = board_path.join(&slug);
    let mut suffix = 2;
    while path.exists() {
        path = board_path.join(format!("{slug}-{suffix}"));
        suffix += 1;
    }

    fs::create_dir(&path)?;
    sync_board_metadata(board_path)?;
    if let Some(mut metadata) = read_board_metadata(board_path)? {
        if let Some(list) = metadata.lists.iter_mut().find(|list| list.id == slug) {
            list.title = name.trim().to_string();
        }
        write_board_metadata(board_path, &metadata)?;
    }
    Ok(path)
}

pub fn rename_list(path: &Path, name: &str) -> io::Result<PathBuf> {
    let board_path = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "list path has no parent"))?
        .to_path_buf();
    sync_board_metadata(&board_path)?;
    let old_id = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "list path has no filename"))?
        .to_string_lossy()
        .to_string();
    let target = unique_path_except(&board_path, &slug_from_title(name), None, path);
    if target != path {
        fs::rename(path, &target)?;
    }
    let new_id = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "list path has no filename"))?
        .to_string_lossy()
        .to_string();
    if let Some(mut metadata) = read_board_metadata(&board_path)? {
        if let Some(list) = metadata.lists.iter_mut().find(|list| list.id == old_id) {
            list.id = new_id;
            list.title = name.trim().to_string();
        }
        write_board_metadata(&board_path, &metadata)?;
    }
    Ok(target)
}

pub fn create_card(list_path: &Path, title: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(list_path)?;
    let mut path = list_path.join(format!("{}.md", slug_from_title(title)));
    let mut suffix = 2;
    while path.exists() {
        path = list_path.join(format!("{}-{}.md", slug_from_title(title), suffix));
        suffix += 1;
    }

    let position = next_card_position(list_path)?;
    fs::write(
        &path,
        markdown_with_position(&format!("# {}\n", title.trim()), position),
    )?;
    Ok(path)
}

pub fn delete_card(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

pub fn rename_card(path: &Path, title: &str) -> io::Result<PathBuf> {
    let list_path = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "card path has no parent"))?;
    let target = unique_path_except(list_path, &slug_from_title(title), Some("md"), path);
    if target != path {
        fs::rename(path, &target)?;
    }
    let content = fs::read_to_string(&target)?;
    fs::write(&target, markdown_with_title(&content, title))?;
    Ok(target)
}

pub fn touch_card_updated_at(path: &Path) -> io::Result<()> {
    let content = fs::read_to_string(path)?;
    fs::write(path, markdown_with_updated_at(&content))
}

pub fn move_card(card_path: &Path, target_list_path: &Path) -> io::Result<PathBuf> {
    let target_index = card_count_excluding(target_list_path, card_path)?;
    move_card_to_index(card_path, target_list_path, target_index)
}

pub fn move_card_to_index(
    card_path: &Path,
    target_list_path: &Path,
    target_index: usize,
) -> io::Result<PathBuf> {
    fs::create_dir_all(target_list_path)?;
    let filename = card_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "card path has no filename"))?;
    let target = target_list_path.join(filename);
    let position = position_for_insert(target_list_path, card_path, target_index)?;
    if card_path != target {
        fs::rename(card_path, &target)?;
    }
    let content = fs::read_to_string(&target)?;
    fs::write(&target, markdown_with_position(&content, position))?;
    Ok(target)
}

pub fn move_list_to_index(
    board_path: &Path,
    list_path: &Path,
    target_index: usize,
) -> io::Result<()> {
    sync_board_metadata(board_path)?;
    let Some(mut metadata) = read_board_metadata(board_path)? else {
        return Ok(());
    };
    let Some(list_id) = list_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return Ok(());
    };
    let Some(source_index) = metadata.lists.iter().position(|list| list.id == list_id) else {
        return Ok(());
    };

    let list = metadata.lists.remove(source_index);
    let target_index = target_index.min(metadata.lists.len());
    metadata.lists.insert(target_index, list);
    for (index, list) in metadata.lists.iter_mut().enumerate() {
        list.position = (index as i64 + 1) * POSITION_STEP;
    }
    write_board_metadata(board_path, &metadata)
}

pub fn board_preview_setting(board_path: &Path) -> io::Result<bool> {
    sync_board_metadata(board_path)?;
    Ok(read_board_metadata(board_path)?
        .map(|metadata| metadata.show_preview)
        .unwrap_or(false))
}

pub fn set_board_preview(board_path: &Path, show_preview: bool) -> io::Result<()> {
    sync_board_metadata(board_path)?;
    let Some(mut metadata) = read_board_metadata(board_path)? else {
        return Ok(());
    };
    metadata.show_preview = show_preview;
    write_board_metadata(board_path, &metadata)
}

fn read_board_metadata(board_path: &Path) -> io::Result<Option<BoardMetadata>> {
    let path = board_path.join(BOARD_METADATA_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    Ok(parse_board_metadata(&content))
}

fn parse_board_metadata(content: &str) -> Option<BoardMetadata> {
    let mut name = None;
    let mut show_preview = false;
    let mut lists = Vec::new();
    let mut current: Option<ListMetadata> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("name:") {
            name = Some(value.trim().to_string());
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("show_preview:") {
            show_preview = value.trim().eq_ignore_ascii_case("true");
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("- id:") {
            if let Some(list) = current.take() {
                lists.push(list);
            }
            let id = value.trim().to_string();
            current = Some(ListMetadata {
                title: id.clone(),
                id,
                position: i64::MAX,
            });
            continue;
        }

        if let Some(list) = current.as_mut() {
            if let Some(value) = trimmed.strip_prefix("title:") {
                list.title = value.trim().to_string();
            } else if let Some(value) = trimmed.strip_prefix("position:") {
                if let Ok(position) = value.trim().parse() {
                    list.position = position;
                }
            }
        }
    }

    if let Some(list) = current {
        lists.push(list);
    }

    Some(BoardMetadata {
        name: name?,
        show_preview,
        lists,
    })
}

fn write_board_metadata(board_path: &Path, metadata: &BoardMetadata) -> io::Result<()> {
    let mut content = format!(
        "name: {}\nshow_preview: {}\n\nlists:\n",
        metadata.name, metadata.show_preview
    );
    for list in &metadata.lists {
        content.push_str(&format!(
            "  - id: {}\n    title: {}\n    position: {}\n\n",
            list.id, list.title, list.position
        ));
    }
    fs::write(board_path.join(BOARD_METADATA_FILE), content)
}

fn unique_path_except(
    parent: &Path,
    slug: &str,
    extension: Option<&str>,
    except: &Path,
) -> PathBuf {
    let mut suffix = 1;
    loop {
        let filename = if suffix == 1 {
            slug.to_string()
        } else {
            format!("{slug}-{suffix}")
        };
        let path = match extension {
            Some(extension) => parent.join(format!("{filename}.{extension}")),
            None => parent.join(filename),
        };
        if path == except || !path.exists() {
            return path;
        }
        suffix += 1;
    }
}

fn sync_board_metadata(board_path: &Path) -> io::Result<()> {
    let existing = read_board_metadata(board_path)?;
    let board_name = existing
        .as_ref()
        .map(|metadata| metadata.name.clone())
        .or_else(|| {
            board_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "Board".to_string());
    let mut lists = Vec::new();
    let mut max_position = existing
        .as_ref()
        .and_then(|metadata| metadata.lists.iter().map(|list| list.position).max())
        .unwrap_or(0);

    let mut dirs = list_directories(board_path)?;
    dirs.sort();
    for id in dirs {
        if let Some(existing_list) = existing
            .as_ref()
            .and_then(|metadata| metadata.lists.iter().find(|list| list.id == id))
        {
            lists.push(existing_list.clone());
        } else {
            max_position += POSITION_STEP;
            lists.push(ListMetadata {
                title: id.clone(),
                id,
                position: max_position,
            });
        }
    }
    lists.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.id.cmp(&right.id))
    });

    write_board_metadata(
        board_path,
        &BoardMetadata {
            name: board_name,
            show_preview: existing
                .as_ref()
                .map(|metadata| metadata.show_preview)
                .unwrap_or(false),
            lists,
        },
    )
}

fn list_directories(board_path: &Path) -> io::Result<Vec<String>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(board_path)? {
        let entry = entry?;
        if entry.path().is_dir() {
            dirs.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    Ok(dirs)
}

fn markdown_files_in(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(files)
}

fn list_sort_position(metadata: Option<&BoardMetadata>, list: &List) -> i64 {
    let Some(metadata) = metadata else {
        return i64::MAX;
    };
    let Some(id) = list.path.file_name().map(|name| name.to_string_lossy()) else {
        return i64::MAX;
    };
    metadata
        .lists
        .iter()
        .find(|metadata_list| metadata_list.id == id)
        .map(|metadata_list| metadata_list.position)
        .unwrap_or(i64::MAX)
}

fn next_card_position(list_path: &Path) -> io::Result<i64> {
    let mut max_position = 0;
    for entry in fs::read_dir(list_path)? {
        let entry = entry?;
        let card_path = entry.path();
        if card_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let content = fs::read_to_string(card_path)?;
        let card = Card::from_markdown(String::new(), PathBuf::new(), content);
        if card.position != i64::MAX {
            max_position = max_position.max(card.position);
        }
    }
    Ok(max_position + POSITION_STEP)
}

fn card_count_excluding(list_path: &Path, excluded_path: &Path) -> io::Result<usize> {
    Ok(sorted_cards_for_positioning(list_path, excluded_path)?.len())
}

fn position_for_insert(
    list_path: &Path,
    excluded_path: &Path,
    target_index: usize,
) -> io::Result<i64> {
    let cards = sorted_cards_for_positioning(list_path, excluded_path)?;
    let target_index = target_index.min(cards.len());
    let before = target_index
        .checked_sub(1)
        .and_then(|index| cards.get(index))
        .map(|card| card.position);
    let after = cards.get(target_index).map(|card| card.position);

    Ok(match (before, after) {
        (None, None) => POSITION_STEP,
        (None, Some(after)) => after - POSITION_STEP,
        (Some(before), None) => before + POSITION_STEP,
        (Some(before), Some(after)) if after - before > 1 => (before + after) / 2,
        (Some(before), Some(_)) => before + 1,
    })
}

fn sorted_cards_for_positioning(list_path: &Path, excluded_path: &Path) -> io::Result<Vec<Card>> {
    let mut cards = Vec::new();
    if !list_path.exists() {
        return Ok(cards);
    }

    for entry in fs::read_dir(list_path)? {
        let entry = entry?;
        let card_path = entry.path();
        if card_path == excluded_path {
            continue;
        }
        if card_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let content = fs::read_to_string(&card_path)?;
        let filename = entry.file_name().to_string_lossy().to_string();
        cards.push(Card::from_markdown(filename, card_path, content));
    }
    cards.sort_by(|left, right| {
        left.position.cmp(&right.position).then_with(|| {
            left.filename
                .to_lowercase()
                .cmp(&right.filename.to_lowercase())
        })
    });
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("niffler-test-{id}"))
    }

    #[test]
    fn load_projects_returns_directories_sorted_by_name() {
        let root = temp_root();
        fs::create_dir_all(root.join("zeta")).unwrap();
        fs::create_dir_all(root.join("alpha")).unwrap();
        fs::write(root.join("ignored.md"), "").unwrap();

        let projects = load_projects(&root).unwrap();

        assert_eq!(
            projects
                .iter()
                .map(|project| project.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn load_board_reads_lists_and_markdown_cards() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("done")).unwrap();
        fs::write(root.join("todo/learn-kzg.md"), "# Learn KZG\nbody").unwrap();
        fs::write(root.join("todo/notes.txt"), "ignored").unwrap();

        let board = load_board(&root).unwrap();

        assert_eq!(board.lists.len(), 2);
        assert_eq!(board.lists[1].name, "todo");
        assert_eq!(board.lists[1].cards[0].title, "Learn KZG");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn load_board_adds_missing_card_metadata() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::write(root.join("todo/task.md"), "# Task\nBody\n").unwrap();

        let board = load_board(&root).unwrap();

        assert_eq!(board.lists[0].cards[0].position, 1000);
        let content = fs::read_to_string(root.join("todo/task.md")).unwrap();
        assert!(content.starts_with("---\nposition: 1000\n"));
        assert!(content.contains("\ncreated_at: "));
        assert!(content.contains("\nupdated_at: "));
        assert!(content.ends_with("---\n\n# Task\nBody\n"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn load_board_initializes_missing_metadata_from_existing_lists() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("doing")).unwrap();

        let board = load_board(&root).unwrap();

        let metadata = fs::read_to_string(root.join(".niffler.yaml")).unwrap();
        assert!(metadata.contains(&format!(
            "name: {}",
            root.file_name().unwrap().to_string_lossy()
        )));
        assert!(metadata.contains("show_preview: false"));
        assert!(metadata.contains("id: doing"));
        assert!(metadata.contains("id: todo"));
        assert_eq!(board.lists.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn load_board_adds_default_preview_setting_to_old_metadata() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::write(
            root.join(".niffler.yaml"),
            "name: Work Board\n\nlists:\n  - id: todo\n    title: TODO\n    position: 1000\n",
        )
        .unwrap();

        load_board(&root).unwrap();

        let metadata = fs::read_to_string(root.join(".niffler.yaml")).unwrap();
        assert!(metadata.contains("show_preview: false"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn board_preview_setting_reads_and_writes_metadata() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::write(
            root.join(".niffler.yaml"),
            "name: Work Board\nshow_preview: true\n\nlists:\n  - id: todo\n    title: TODO\n    position: 1000\n",
        )
        .unwrap();

        assert!(board_preview_setting(&root).unwrap());

        set_board_preview(&root, false).unwrap();

        assert!(!board_preview_setting(&root).unwrap());
        let metadata = fs::read_to_string(root.join(".niffler.yaml")).unwrap();
        assert!(metadata.contains("show_preview: false"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn load_board_uses_metadata_for_board_name_and_list_order() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("doing")).unwrap();
        fs::write(
            root.join(".niffler.yaml"),
            "name: Work Board\n\nlists:\n  - id: doing\n    title: DOING\n    position: 2000\n\n  - id: todo\n    title: TODO\n    position: 1000\n",
        )
        .unwrap();

        let board = load_board(&root).unwrap();

        assert_eq!(board.name, "Work Board");
        assert_eq!(
            board
                .lists
                .iter()
                .map(|list| list.name.as_str())
                .collect::<Vec<_>>(),
            vec!["TODO", "DOING"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn load_board_sorts_cards_by_frontmatter_position() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::write(
            root.join("todo/last.md"),
            "---\nposition: 3000\n---\n\n# Last\n",
        )
        .unwrap();
        fs::write(
            root.join("todo/first.md"),
            "---\nposition: 1000\n---\n\n# First\n",
        )
        .unwrap();

        let board = load_board(&root).unwrap();

        assert_eq!(
            board.lists[0]
                .cards
                .iter()
                .map(|card| card.title.as_str())
                .collect::<Vec<_>>(),
            vec!["First", "Last"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_project_writes_unique_slugged_board_directory() {
        let root = temp_root();

        let first = create_project(&root, "Work Board").unwrap();
        let second = create_project(&root, "Work Board").unwrap();

        assert_eq!(first.file_name().unwrap(), "work-board");
        assert_eq!(second.file_name().unwrap(), "work-board-2");
        assert!(first.is_dir());
        assert!(second.is_dir());
        assert!(first.join(".niffler.yaml").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_project_removes_board_directory_and_contents() {
        let root = temp_root();
        fs::create_dir_all(root.join("work/todo")).unwrap();
        fs::write(root.join("work/todo/task.md"), "# Task\n").unwrap();

        delete_project(&root.join("work")).unwrap();

        assert!(!root.join("work").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_list_removes_list_directory_and_cards() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::write(root.join("todo/task.md"), "# Task\n").unwrap();

        delete_list(&root.join("todo")).unwrap();

        assert!(!root.join("todo").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_list_writes_unique_slugged_directory_inside_board() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();

        let first = create_list(&root, "In Progress").unwrap();
        let second = create_list(&root, "In Progress").unwrap();

        assert_eq!(first.file_name().unwrap(), "in-progress");
        assert_eq!(second.file_name().unwrap(), "in-progress-2");
        assert!(first.is_dir());
        assert!(second.is_dir());
        let metadata = fs::read_to_string(root.join(".niffler.yaml")).unwrap();
        assert!(metadata.contains("id: in-progress"));
        assert!(metadata.contains("id: in-progress-2"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_list_to_index_updates_metadata_order() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("doing")).unwrap();
        fs::create_dir_all(root.join("done")).unwrap();
        load_board(&root).unwrap();

        move_list_to_index(&root, &root.join("done"), 0).unwrap();

        let board = load_board(&root).unwrap();
        assert_eq!(
            board
                .lists
                .iter()
                .map(|list| list.path.file_name().unwrap().to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["done", "doing", "todo"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_card_writes_markdown_file_with_unique_slug() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();

        let first = create_card(&root.join("todo"), "Learn KZG").unwrap();
        let second = create_card(&root.join("todo"), "Learn KZG").unwrap();

        assert_eq!(first.file_name().unwrap(), "learn-kzg.md");
        assert_eq!(second.file_name().unwrap(), "learn-kzg-2.md");
        let content = fs::read_to_string(first).unwrap();
        assert!(content.starts_with("---\nposition: 1000\n"));
        assert!(content.contains("\ncreated_at: "));
        assert!(content.contains("\nupdated_at: "));
        assert!(content.ends_with("---\n\n# Learn KZG\n"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_card_renames_file_into_target_list() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("doing")).unwrap();
        let original = root.join("todo/learn.md");
        fs::write(&original, "# Learn\n").unwrap();

        let moved = move_card(&original, &root.join("doing")).unwrap();

        assert!(!original.exists());
        assert_eq!(moved, root.join("doing/learn.md"));
        assert!(moved.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_card_to_index_updates_position_between_neighbors() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::write(root.join("todo/a.md"), "---\nposition: 1000\n---\n\n# A\n").unwrap();
        fs::write(root.join("todo/b.md"), "---\nposition: 2000\n---\n\n# B\n").unwrap();
        fs::write(root.join("todo/c.md"), "---\nposition: 3000\n---\n\n# C\n").unwrap();

        move_card_to_index(&root.join("todo/c.md"), &root.join("todo"), 1).unwrap();

        let board = load_board(&root).unwrap();
        assert_eq!(
            board.lists[0]
                .cards
                .iter()
                .map(|card| (card.title.as_str(), card.position))
                .collect::<Vec<_>>(),
            vec![("A", 1000), ("C", 1500), ("B", 2000)]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_card_to_index_between_lists_can_insert_at_top() {
        let root = temp_root();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("doing")).unwrap();
        fs::write(
            root.join("todo/build.md"),
            "---\nposition: 2000\n---\n\n# Build\n",
        )
        .unwrap();
        fs::write(
            root.join("doing/refactor.md"),
            "---\nposition: 1000\n---\n\n# Refactor\n",
        )
        .unwrap();

        let moved =
            move_card_to_index(&root.join("todo/build.md"), &root.join("doing"), 0).unwrap();

        assert_eq!(moved, root.join("doing/build.md"));
        let board = load_board(&root).unwrap();
        let doing = board
            .lists
            .iter()
            .find(|list| list.path.ends_with("doing"))
            .unwrap();
        assert_eq!(
            doing
                .cards
                .iter()
                .map(|card| (card.title.as_str(), card.position))
                .collect::<Vec<_>>(),
            vec![("Build", 0), ("Refactor", 1000)]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
