use autocorrect_app_lib::commands::spellcheck::{spell_check, SpellCheckResult};

fn main() {
    env_logger::init();

    let test_text = "hows areyour and funciton";

    println!("Testing spell_check with: '{}'", test_text);
    println!("");

    match spell_check(test_text.to_string()) {
        Ok(result) => {
            println!("✅ Spell check completed");
            println!("");
            println!("Original: {}", result.original);
            println!("Corrected: {}", result.corrected);
            println!("Has changes: {}", result.has_changes);
            println!("");
            println!("Typos found: {}", result.typos.len());
            for typo in result.typos {
                println!("  - '{}' at line {} col {}", typo.typo, typo.line, typo.col);
                if !typo.suggestions.is_empty() {
                    println!("    Suggestions: {:?}", typo.suggestions);
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }
}
