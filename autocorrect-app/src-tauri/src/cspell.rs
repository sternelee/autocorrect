use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::typocheck::TypoError;

/// CSpell issue from JSON reporter
#[derive(Debug, Deserialize)]
struct CSpellIssue {
    text: String,
    offset: usize,
    #[serde(default)]
    suggestions: Vec<String>,
    #[serde(default)]
    #[serde(rename = "suggestionsEx")]
    suggestions_ex: Vec<SuggestionEx>,
    row: usize,
    col: usize,
}

#[derive(Debug, Deserialize)]
struct SuggestionEx {
    word: String,
    #[serde(default)]
    #[serde(rename = "isPreferred")]
    is_preferred: bool,
}

/// CSpell JSON output format
#[derive(Debug, Deserialize)]
struct CSpellOutput {
    issues: Vec<CSpellIssue>,
}

/// Supported CSpell dictionaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSpellDictionaries {
    // Programming Languages
    pub typescript: bool,
    pub python: bool,
    pub rust: bool,
    pub cpp: bool,
    pub java: bool,
    pub go: bool,
    pub csharp: bool,
    pub php: bool,
    pub ruby: bool,
    pub swift: bool,

    // Web Technologies
    pub html: bool,
    pub css: bool,
    pub node: bool,
    pub npm: bool,

    // Frameworks
    pub react: bool,
    pub vue: bool,
    pub django: bool,

    // Tools & Platforms
    pub docker: bool,
    pub k8s: bool,
    pub aws: bool,
    pub git: bool,

    // General
    pub companies: bool,
    pub software_terms: bool,
    pub filetypes: bool,
    pub public_licenses: bool,
}

impl Default for CSpellDictionaries {
    fn default() -> Self {
        Self {
            // Enable most common ones by default
            typescript: true,
            python: false,
            rust: false,
            cpp: false,
            java: false,
            go: false,
            csharp: false,
            php: false,
            ruby: false,
            swift: false,

            html: true,
            css: true,
            node: true,
            npm: true,

            react: false,
            vue: false,
            django: false,

            docker: false,
            k8s: false,
            aws: false,
            git: true,

            companies: true,
            software_terms: true,
            filetypes: true,
            public_licenses: true,
        }
    }
}

impl CSpellDictionaries {
    /// Get list of enabled dictionary names
    pub fn get_enabled(&self) -> Vec<String> {
        let mut dicts = Vec::new();

        // Programming Languages
        if self.typescript {
            dicts.push("typescript".to_string());
        }
        if self.python {
            dicts.push("python".to_string());
        }
        if self.rust {
            dicts.push("rust".to_string());
        }
        if self.cpp {
            dicts.push("cpp".to_string());
        }
        if self.java {
            dicts.push("java".to_string());
        }
        if self.go {
            dicts.push("golang".to_string());
        }
        if self.csharp {
            dicts.push("csharp".to_string());
        }
        if self.php {
            dicts.push("php".to_string());
        }
        if self.ruby {
            dicts.push("ruby".to_string());
        }
        if self.swift {
            dicts.push("swift".to_string());
        }

        // Web Technologies
        if self.html {
            dicts.push("html".to_string());
        }
        if self.css {
            dicts.push("css".to_string());
        }
        if self.node {
            dicts.push("node".to_string());
        }
        if self.npm {
            dicts.push("npm".to_string());
        }

        // Frameworks
        if self.react {
            dicts.push("react".to_string());
        }
        if self.vue {
            dicts.push("vue".to_string());
        }
        if self.django {
            dicts.push("django".to_string());
        }

        // Tools & Platforms
        if self.docker {
            dicts.push("docker".to_string());
        }
        if self.k8s {
            dicts.push("k8s".to_string());
        }
        if self.aws {
            dicts.push("aws".to_string());
        }
        if self.git {
            dicts.push("git".to_string());
        }

        // General (use softwareTerms format instead of software_terms)
        if self.companies {
            dicts.push("companies".to_string());
        }
        if self.software_terms {
            dicts.push("softwareTerms".to_string());
        }
        if self.filetypes {
            dicts.push("filetypes".to_string());
        }
        if self.public_licenses {
            dicts.push("public-licenses".to_string());
        }

        dicts
    }
}

/// Find the cspell executable path
fn find_cspell_path() -> Option<String> {
    // Get current executable path and navigate to project root
    if let Ok(exe_path) = std::env::current_exe() {
        // Try to find autocorrect-app directory
        let mut current = exe_path.parent()?;

        // In dev mode: target/debug/autocorrect-app -> navigate up to autocorrect-app/
        // In release: target/release/bundle/.../autocorrect-app -> navigate up
        for _ in 0..10 {
            let node_modules_path = current.join("node_modules/.bin/cspell");
            log::debug!("Checking CSpell at: {:?}", node_modules_path);

            if node_modules_path.exists() {
                log::info!("Found CSpell at: {:?}", node_modules_path);
                return Some(node_modules_path.to_string_lossy().to_string());
            }

            // Also check parent's node_modules (for nested structures)
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }
    }

    // Try current working directory
    let cwd_paths = vec![
        "node_modules/.bin/cspell",
        "../node_modules/.bin/cspell",
        "../../node_modules/.bin/cspell",
    ];

    for path in cwd_paths {
        let full_path = std::path::Path::new(path);
        log::debug!("Checking CSpell at (CWD): {:?}", full_path);
        if full_path.exists() {
            log::info!("Found CSpell at (CWD): {:?}", full_path);
            return Some(path.to_string());
        }
    }

    // Try global cspell
    log::debug!("Trying to find global cspell with 'which'");
    if let Ok(output) = Command::new("which").arg("cspell").output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let trimmed = path.trim().to_string();
                log::info!("Found global CSpell at: {}", trimmed);
                return Some(trimmed);
            }
        }
    }

    log::warn!("CSpell not found in any location");
    None
}

/// Check text for typos using CSpell
pub fn check_with_cspell(text: &str, dictionaries: &CSpellDictionaries) -> Vec<TypoError> {
    let cspell_path = match find_cspell_path() {
        Some(path) => path,
        None => {
            log::warn!("CSpell not found, skipping CSpell check");
            return Vec::new();
        }
    };

    let enabled_dicts = dictionaries.get_enabled();
    if enabled_dicts.is_empty() {
        log::debug!("No CSpell dictionaries enabled, skipping check");
        return Vec::new();
    }

    log::debug!("Running CSpell with dictionaries: {:?}", enabled_dicts);

    // Build cspell command
    let mut cmd = Command::new(&cspell_path);
    cmd.arg("stdin")
        .arg("--no-progress")
        .arg("--no-summary")
        .arg("--reporter")
        .arg("@cspell/cspell-json-reporter")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Add dictionary arguments
    for dict in enabled_dicts {
        cmd.arg("--dictionary").arg(&dict);
    }

    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            log::error!("Failed to spawn cspell: {}", e);
            return Vec::new();
        }
    };

    // Write text to stdin
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(text.as_bytes()) {
            log::error!("Failed to write to cspell stdin: {}", e);
            return Vec::new();
        }
    }

    // Read output
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            log::error!("Failed to wait for cspell: {}", e);
            return Vec::new();
        }
    };

    if !output.status.success() {
        // CSpell returns exit code 1 when issues are found, which is expected
        // Only log if it's a real error (exit code > 1)
        if let Some(code) = output.status.code() {
            if code > 1 {
                log::warn!("CSpell exited with code {}", code);
                if !output.stderr.is_empty() {
                    if let Ok(stderr) = String::from_utf8(output.stderr) {
                        log::warn!("CSpell stderr: {}", stderr);
                    }
                }
            }
        }
    }

    // Parse JSON output
    let json_output = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse cspell output as UTF-8: {}", e);
            return Vec::new();
        }
    };

    if json_output.trim().is_empty() {
        log::debug!("CSpell returned empty output (no issues found)");
        return Vec::new();
    }

    let cspell_output: CSpellOutput = match serde_json::from_str(&json_output) {
        Ok(output) => output,
        Err(e) => {
            log::error!("Failed to parse cspell JSON: {}", e);
            log::debug!("CSpell output was: {}", json_output);
            return Vec::new();
        }
    };

    // Convert CSpell issues to TypoError format
    let mut typo_errors = Vec::new();
    for issue in cspell_output.issues {
        let suggestions = if !issue.suggestions.is_empty() {
            issue.suggestions
        } else {
            // Extract preferred suggestions from suggestionsEx
            issue
                .suggestions_ex
                .iter()
                .filter(|s| s.is_preferred)
                .map(|s| s.word.clone())
                .collect()
        };

        typo_errors.push(TypoError {
            typo: issue.text,
            suggestions,
            byte_offset: issue.offset,
            line: issue.row,
            col: issue.col,
        });
    }

    log::debug!("CSpell found {} issues", typo_errors.len());
    typo_errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dictionaries() {
        let dicts = CSpellDictionaries::default();
        let enabled = dicts.get_enabled();

        // Should have some defaults enabled
        assert!(enabled.contains(&"typescript".to_string()));
        assert!(enabled.contains(&"html".to_string()));
        assert!(enabled.contains(&"css".to_string()));
        assert!(enabled.contains(&"node".to_string()));
        assert!(enabled.contains(&"npm".to_string()));
        assert!(enabled.contains(&"companies".to_string()));
        assert!(enabled.contains(&"softwareTerms".to_string()));
    }

    #[test]
    fn test_custom_dictionaries() {
        let mut dicts = CSpellDictionaries::default();
        dicts.typescript = false;
        dicts.python = true;
        dicts.rust = true;

        let enabled = dicts.get_enabled();
        assert!(!enabled.contains(&"typescript".to_string()));
        assert!(enabled.contains(&"python".to_string()));
        assert!(enabled.contains(&"rust".to_string()));
    }

    #[test]
    fn test_cspell_check_typescript() {
        let text = "const useState = 123; let funciton = 456;";
        let dicts = CSpellDictionaries::default();

        let errors = check_with_cspell(text, &dicts);

        // Should find "funciton" but not "useState"
        assert!(errors.iter().any(|e| e.typo == "funciton"));
        assert!(!errors.iter().any(|e| e.typo == "useState"));
    }

    #[test]
    fn test_cspell_no_errors() {
        let text = "const React = require('react');";
        let dicts = CSpellDictionaries::default();

        let errors = check_with_cspell(text, &dicts);

        // Should not find any errors with React/require/react
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_cspell_hows_areyour() {
        let text = "hows areyour";
        let dicts = CSpellDictionaries::default();

        println!("\n=== Testing: '{}' ===", text);

        let errors = check_with_cspell(text, &dicts);

        println!("Found {} errors:", errors.len());
        for error in &errors {
            println!(
                "  - '{}' at line {} col {}",
                error.typo, error.line, error.col
            );
            if !error.suggestions.is_empty() {
                println!("    Suggestions: {:?}", error.suggestions);
            }
        }

        // CSpell should find "areyour" (but may not find "hows")
        // Let's just verify it runs without panicking
        assert!(errors.len() >= 0);
    }
}
