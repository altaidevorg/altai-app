//! List installed workspace skills for the stdio host (read-only catalogue).

use std::path::Path;

use serde_json::{json, Value};

/// Skills live under `<workspace>/.isanagent/skills/<name>/`.
pub fn list_workspace_skills(workspace_root: &Path) -> Result<Value, String> {
    let skills_dir = workspace_root.join(".isanagent").join("skills");
    let entries = match std::fs::read_dir(&skills_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({ "skills": [] }));
        }
        Err(error) => return Err(error.to_string()),
    };

    let mut skills = entries
        .flatten()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            if !kind.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().trim().to_string();
            if name.is_empty() || name.starts_with('.') {
                return None;
            }
            let description = std::fs::read_to_string(entry.path().join("SKILL.md"))
                .ok()
                .and_then(|text| skill_description(&text));
            Some(json!({
                "name": name,
                "description": description,
                "enabled": true,
            }))
        })
        .collect::<Vec<_>>();
    skills.sort_by(|a, b| {
        let an = a.get("name").and_then(Value::as_str).unwrap_or("");
        let bn = b.get("name").and_then(Value::as_str).unwrap_or("");
        an.to_lowercase().cmp(&bn.to_lowercase())
    });
    Ok(json!({ "skills": skills }))
}

fn skill_description(text: &str) -> Option<String> {
    for line in text.lines().take(40) {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("description:") {
            let value = value.trim().trim_matches(['\'', '"']);
            if !value.is_empty() {
                return Some(value.chars().take(180).collect());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn lists_skills_from_workspace() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join(".isanagent").join("skills").join("demo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: Demo skill\n---\n",
        )
        .unwrap();
        let result = list_workspace_skills(dir.path()).unwrap();
        let skills = result["skills"].as_array().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0]["name"], "demo");
        assert_eq!(skills[0]["description"], "Demo skill");
    }

    #[test]
    fn empty_when_missing_dir() {
        let dir = tempdir().unwrap();
        let result = list_workspace_skills(dir.path()).unwrap();
        assert_eq!(result["skills"].as_array().unwrap().len(), 0);
    }
}
