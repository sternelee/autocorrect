#!/bin/bash

# Test script for CSpell integration

echo "=== CSpell Integration Test ==="
echo ""

cd "$(dirname "$0")/src-tauri"

echo "1. Testing CSpell path finding..."
cargo test --lib cspell::tests::test_cspell_check_typescript -- --nocapture 2>&1 | grep -A 5 "test_cspell"

echo ""
echo "2. Testing with 'hows areyour' text..."
cat > /tmp/test_cspell_integration.rs << 'EOF'
#[cfg(test)]
mod integration_test {
    use crate::cspell::{check_with_cspell, CSpellDictionaries};

    #[test]
    fn test_hows_areyour() {
        let text = "hows areyour";
        let dicts = CSpellDictionaries::default();
        
        println!("\n=== Testing: '{}' ===", text);
        println!("Enabled dictionaries: {:?}", dicts.get_enabled());
        
        let errors = check_with_cspell(text, &dicts);
        
        println!("Found {} errors:", errors.len());
        for error in &errors {
            println!("  - '{}' at line {} col {}", error.typo, error.line, error.col);
            if !error.suggestions.is_empty() {
                println!("    Suggestions: {:?}", error.suggestions);
            }
        }
        
        // Should find at least "areyour"
        assert!(errors.len() > 0, "Expected to find errors in 'hows areyour'");
    }
}
EOF

# Append test to cspell.rs temporarily
echo "" >> src/cspell.rs
cat /tmp/test_cspell_integration.rs >> src/cspell.rs

echo "Running integration test..."
RUST_LOG=debug cargo test --lib integration_test::test_hows_areyour -- --nocapture 2>&1 | grep -A 20 "=== Testing:"

# Restore original file
cd ../..
git checkout autocorrect-app/src-tauri/src/cspell.rs 2>/dev/null

echo ""
echo "=== Test Complete ==="
