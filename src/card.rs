use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Card {
    pub title: String,
    pub filename: String,
    pub path: PathBuf,
    pub content: String,
    pub position: i64,
}

impl Card {
    pub fn from_markdown(filename: String, path: PathBuf, content: String) -> Self {
        let title = title_from_markdown(&content).unwrap_or_else(|| title_from_filename(&filename));
        let position = position_from_markdown(&content).unwrap_or(i64::MAX);

        Self {
            title,
            filename,
            path,
            content,
            position,
        }
    }
}

pub fn position_from_markdown(content: &str) -> Option<i64> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            return None;
        }
        if let Some(value) = trimmed.strip_prefix("position:") {
            return value.trim().parse().ok();
        }
    }

    None
}

pub fn markdown_with_position(content: &str, position: i64) -> String {
    let normalized = content.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let now = current_timestamp();
    if lines.first().is_some_and(|line| line.trim() == "---") {
        if let Some(end_index) = lines.iter().skip(1).position(|line| line.trim() == "---") {
            let end_index = end_index + 1;
            let mut frontmatter = lines[1..end_index]
                .iter()
                .map(|line| (*line).to_string())
                .collect::<Vec<_>>();
            upsert_frontmatter_field(&mut frontmatter, "position", position.to_string());
            ensure_frontmatter_field(&mut frontmatter, "created_at", now.to_string());
            upsert_frontmatter_field(&mut frontmatter, "updated_at", now.to_string());

            let body = lines[end_index + 1..].join("\n");
            return format!(
                "---\n{}\n---\n{}",
                frontmatter.join("\n"),
                normalize_body(&body)
            );
        }
    }

    format!(
        "---\nposition: {position}\ncreated_at: {now}\nupdated_at: {now}\n---\n{}",
        normalize_body(&normalized)
    )
}

pub fn markdown_with_missing_metadata(content: &str, default_position: i64) -> String {
    let normalized = content.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let now = current_timestamp();
    if lines.first().is_some_and(|line| line.trim() == "---") {
        if let Some(end_index) = lines.iter().skip(1).position(|line| line.trim() == "---") {
            let end_index = end_index + 1;
            let mut frontmatter = lines[1..end_index]
                .iter()
                .map(|line| (*line).to_string())
                .collect::<Vec<_>>();
            if frontmatter_has_field(&frontmatter, "position")
                && frontmatter_has_field(&frontmatter, "created_at")
                && frontmatter_has_field(&frontmatter, "updated_at")
            {
                return normalized;
            }
            ensure_frontmatter_field(&mut frontmatter, "position", default_position.to_string());
            ensure_frontmatter_field(&mut frontmatter, "created_at", now.to_string());
            ensure_frontmatter_field(&mut frontmatter, "updated_at", now.to_string());

            let body = lines[end_index + 1..].join("\n");
            return format!(
                "---\n{}\n---\n{}",
                frontmatter.join("\n"),
                normalize_body(&body)
            );
        }
    }

    format!(
        "---\nposition: {default_position}\ncreated_at: {now}\nupdated_at: {now}\n---\n{}",
        normalize_body(&normalized)
    )
}

pub fn preview_content(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    if lines.first().is_some_and(|line| line.trim() == "---") {
        if let Some(end_index) = lines.iter().skip(1).position(|line| line.trim() == "---") {
            let end_index = end_index + 1;
            let body = lines[end_index + 1..].join("\n");
            let body = body.trim_start_matches('\n').to_string();
            if normalized.ends_with('\n') && !body.ends_with('\n') {
                return format!("{body}\n");
            }
            return body;
        }
    }
    normalized
}

fn normalize_body(body: &str) -> String {
    let trimmed = body.trim_start_matches('\n');
    if trimmed.is_empty() {
        "\n".to_string()
    } else {
        format!("\n{trimmed}")
    }
}

pub fn title_from_markdown(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("# ")
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(ToOwned::to_owned)
    })
}

pub fn markdown_with_title(content: &str, title: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let title_line = format!("# {}", title.trim());
    let mut replaced = false;
    let mut lines = normalized
        .lines()
        .map(|line| {
            if !replaced && line.trim_start().starts_with("# ") {
                replaced = true;
                title_line.clone()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();

    if !replaced {
        if let Some(end_index) = frontmatter_end_index(&lines) {
            let insert_index = end_index + 1;
            lines.insert(insert_index, String::new());
            lines.insert(insert_index + 1, title_line);
        } else {
            lines.insert(0, title_line);
            lines.insert(1, String::new());
        }
    }

    touch_updated_at(&mut lines);

    let mut output = lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }
    output
}

pub fn markdown_with_updated_at(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let mut lines = normalized
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    touch_updated_at(&mut lines);
    let mut output = lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn touch_updated_at(lines: &mut Vec<String>) {
    let now = current_timestamp().to_string();
    if let Some(end_index) = frontmatter_end_index(lines) {
        let mut frontmatter = lines[1..end_index].to_vec();
        ensure_frontmatter_field(&mut frontmatter, "created_at", now.clone());
        upsert_frontmatter_field(&mut frontmatter, "updated_at", now);
        lines.splice(1..end_index, frontmatter);
    } else {
        lines.insert(0, "---".to_string());
        lines.insert(1, format!("created_at: {now}"));
        lines.insert(2, format!("updated_at: {now}"));
        lines.insert(3, "---".to_string());
        lines.insert(4, String::new());
    }
}

fn ensure_frontmatter_field(frontmatter: &mut Vec<String>, key: &str, value: String) {
    if !frontmatter_has_field(frontmatter, key) {
        frontmatter.push(format!("{key}: {value}"));
    }
}

fn upsert_frontmatter_field(frontmatter: &mut Vec<String>, key: &str, value: String) {
    if let Some(line) = frontmatter
        .iter_mut()
        .find(|line| frontmatter_line_has_key(line, key))
    {
        *line = format!("{key}: {value}");
    } else {
        frontmatter.push(format!("{key}: {value}"));
    }
}

fn frontmatter_has_field(frontmatter: &[String], key: &str) -> bool {
    frontmatter
        .iter()
        .any(|line| frontmatter_line_has_key(line, key))
}

fn frontmatter_line_has_key(line: &str, key: &str) -> bool {
    line.trim_start().starts_with(&format!("{key}:"))
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn frontmatter_end_index(lines: &[String]) -> Option<usize> {
    if lines.first().is_none_or(|line| line.trim() != "---") {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line.trim() == "---").then_some(index))
}

pub fn title_from_filename(filename: &str) -> String {
    filename
        .trim_end_matches(".md")
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn slug_from_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_title_uses_first_markdown_h1() {
        let title = title_from_markdown("intro\n# Learn KZG\nbody");

        assert_eq!(title.as_deref(), Some("Learn KZG"));
    }

    #[test]
    fn card_title_falls_back_to_humanized_filename() {
        let card = Card::from_markdown(
            "learn-kzg.md".to_string(),
            PathBuf::from("todo/learn-kzg.md"),
            "body only".to_string(),
        );

        assert_eq!(card.title, "Learn Kzg");
    }

    #[test]
    fn parses_position_from_markdown_frontmatter() {
        assert_eq!(
            position_from_markdown("---\nposition: 1500\n---\n\n# Task\n"),
            Some(1500)
        );
        assert_eq!(position_from_markdown("# Task\n"), None);
    }

    #[test]
    fn writes_or_updates_position_frontmatter() {
        let created = markdown_with_position("# Task\n", 1000);
        assert!(created.starts_with("---\nposition: 1000\n"));
        assert!(created.contains("\ncreated_at: "));
        assert!(created.contains("\nupdated_at: "));
        assert!(created.ends_with("---\n\n# Task\n"));

        let updated = markdown_with_position(
            "---\nposition: 1\ncreated_at: 123\nupdated_at: 124\n---\n\n# Task\n",
            2000,
        );
        assert!(updated.starts_with("---\nposition: 2000\n"));
        assert!(updated.contains("\ncreated_at: 123\n"));
        assert!(updated.contains("\nupdated_at: "));
        assert!(updated.ends_with("---\n\n# Task"));
    }

    #[test]
    fn fills_missing_card_metadata_without_replacing_existing_values() {
        let filled = markdown_with_missing_metadata("# Task\n", 1000);
        assert!(filled.starts_with("---\nposition: 1000\n"));
        assert!(filled.contains("\ncreated_at: "));
        assert!(filled.contains("\nupdated_at: "));
        assert!(filled.ends_with("---\n\n# Task\n"));

        let preserved = markdown_with_missing_metadata(
            "---\nposition: 2000\ncreated_at: 123\nupdated_at: 124\n---\n\n# Task\n",
            1000,
        );
        assert_eq!(
            preserved,
            "---\nposition: 2000\ncreated_at: 123\nupdated_at: 124\n---\n\n# Task\n"
        );
    }

    #[test]
    fn card_preview_content_hides_frontmatter() {
        assert_eq!(
            preview_content(
                "---\nposition: 1\ncreated_at: 2\nupdated_at: 3\n---\n\n# Task\nBody\n"
            ),
            "# Task\nBody\n"
        );
        assert_eq!(preview_content("# Task\nBody\n"), "# Task\nBody\n");
    }

    #[test]
    fn slug_from_title_is_filesystem_friendly() {
        assert_eq!(slug_from_title(" Learn KZG!! "), "learn-kzg");
        assert_eq!(slug_from_title("???"), "untitled");
    }

    #[test]
    fn updates_or_inserts_markdown_title() {
        let without_frontmatter = markdown_with_title("# Old\nBody\n", "New");
        assert!(without_frontmatter.starts_with("---\ncreated_at: "));
        assert!(without_frontmatter.contains("\nupdated_at: "));
        assert!(without_frontmatter.ends_with("---\n\n# New\nBody\n"));

        let with_frontmatter = markdown_with_title(
            "---\nposition: 1\ncreated_at: 123\nupdated_at: 124\n---\n\nBody\n",
            "New",
        );
        assert!(with_frontmatter.starts_with("---\nposition: 1\n"));
        assert!(with_frontmatter.contains("\ncreated_at: 123\n"));
        assert!(with_frontmatter.contains("\nupdated_at: "));
        assert!(with_frontmatter.ends_with("---\n\n# New\n\nBody\n"));
    }

    #[test]
    fn updates_card_updated_at_metadata() {
        let updated = markdown_with_updated_at(
            "---\nposition: 1\ncreated_at: 123\nupdated_at: 124\n---\n\n# Task\n",
        );

        assert!(updated.starts_with("---\nposition: 1\n"));
        assert!(updated.contains("\ncreated_at: 123\n"));
        assert!(updated.contains("\nupdated_at: "));
        assert!(updated.ends_with("---\n\n# Task\n"));
    }
}
