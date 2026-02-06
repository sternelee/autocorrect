use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A single custom correction entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomCorrection {
    pub typo: String,
    pub correction: String,
}

/// Get the path to the custom corrections file
fn get_custom_corrections_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".autocorrect-typos.txt"))
}

/// Read all custom corrections from the file
fn read_corrections_file() -> Result<Vec<CustomCorrection>, String> {
    let path = get_custom_corrections_path().ok_or("Could not find home directory")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read corrections file: {}", e))?;

    let mut corrections = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((typo, correction)) = line.split_once('=') {
            corrections.push(CustomCorrection {
                typo: typo.trim().to_string(),
                correction: correction.trim().to_string(),
            });
        }
    }

    Ok(corrections)
}

/// Write corrections to the file
fn write_corrections_file(corrections: &[CustomCorrection]) -> Result<(), String> {
    let path = get_custom_corrections_path().ok_or("Could not find home directory")?;

    let mut content = String::from("# Custom typo corrections for AutoCorrect\n");
    content.push_str("# Format: typo=correction\n");
    content.push_str("#\n");
    content.push_str("# Example:\n");
    content.push_str("# whts=what's\n");
    content.push_str("# teh=the\n");
    content.push_str("#\n");
    content.push_str("# Lines starting with # are comments\n\n");

    for correction in corrections {
        content.push_str(&format!("{}={}\n", correction.typo, correction.correction));
    }

    fs::write(&path, content).map_err(|e| format!("Failed to write corrections file: {}", e))?;

    // Force reload of the custom corrections in typocheck module
    reload_typocheck_corrections();

    Ok(())
}

/// Force reload of custom corrections in typocheck module
fn reload_typocheck_corrections() {
    // Call the reload function from typocheck module
    crate::typocheck::reload_custom_corrections();
    log::info!("Custom corrections reloaded successfully");
}

/// Get all custom corrections
#[tauri::command]
pub fn get_custom_corrections() -> Result<Vec<CustomCorrection>, String> {
    read_corrections_file()
}

/// Add a new custom correction
#[tauri::command]
pub fn add_custom_correction(typo: String, correction: String) -> Result<(), String> {
    if typo.trim().is_empty() {
        return Err("Typo cannot be empty".to_string());
    }
    if correction.trim().is_empty() {
        return Err("Correction cannot be empty".to_string());
    }

    let mut corrections = read_corrections_file()?;

    // Check if typo already exists
    if corrections
        .iter()
        .any(|c| c.typo.to_lowercase() == typo.trim().to_lowercase())
    {
        return Err(format!("Typo '{}' already exists", typo.trim()));
    }

    corrections.push(CustomCorrection {
        typo: typo.trim().to_string(),
        correction: correction.trim().to_string(),
    });

    write_corrections_file(&corrections)?;

    Ok(())
}

/// Update an existing custom correction
#[tauri::command]
pub fn update_custom_correction(
    old_typo: String,
    new_typo: String,
    new_correction: String,
) -> Result<(), String> {
    if new_typo.trim().is_empty() {
        return Err("Typo cannot be empty".to_string());
    }
    if new_correction.trim().is_empty() {
        return Err("Correction cannot be empty".to_string());
    }

    let mut corrections = read_corrections_file()?;

    // Find the correction to update
    let index = corrections
        .iter()
        .position(|c| c.typo.to_lowercase() == old_typo.trim().to_lowercase())
        .ok_or(format!("Typo '{}' not found", old_typo.trim()))?;

    // Check if new typo conflicts with existing (unless it's the same typo)
    if old_typo.to_lowercase() != new_typo.to_lowercase()
        && corrections
            .iter()
            .any(|c| c.typo.to_lowercase() == new_typo.trim().to_lowercase())
    {
        return Err(format!("Typo '{}' already exists", new_typo.trim()));
    }

    corrections[index] = CustomCorrection {
        typo: new_typo.trim().to_string(),
        correction: new_correction.trim().to_string(),
    };

    write_corrections_file(&corrections)?;

    Ok(())
}

/// Delete a custom correction
#[tauri::command]
pub fn delete_custom_correction(typo: String) -> Result<(), String> {
    let mut corrections = read_corrections_file()?;

    let original_len = corrections.len();
    corrections.retain(|c| c.typo.to_lowercase() != typo.trim().to_lowercase());

    if corrections.len() == original_len {
        return Err(format!("Typo '{}' not found", typo.trim()));
    }

    write_corrections_file(&corrections)?;

    Ok(())
}

/// Get the path to the custom corrections file (for display)
#[tauri::command]
pub fn get_custom_corrections_path_cmd() -> Result<String, String> {
    get_custom_corrections_path()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or("Could not find home directory".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_custom_corrections_path() {
        let path = get_custom_corrections_path();
        assert!(path.is_some());
        let path_str = path.unwrap().to_string_lossy().to_string();
        assert!(path_str.contains(".autocorrect-typos.txt"));
    }

    #[test]
    fn test_read_empty_file() {
        // If file doesn't exist, should return empty vec
        // This test assumes the file might not exist
        let result = read_corrections_file();
        assert!(result.is_ok());
    }
}
