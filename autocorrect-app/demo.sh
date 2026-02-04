#!/bin/bash

# AutoCorrect Desktop App - Demo/Test Script
# This script provides a guided tour of the application's features

set -e

echo "=========================================="
echo "AutoCorrect Desktop App - Demo Guide"
echo "=========================================="
echo ""

# Check if we're in the right directory
if [ ! -f "package.json" ] || [ ! -d "src-tauri" ]; then
    echo "Error: Please run this script from the autocorrect-app directory"
    exit 1
fi

echo "Prerequisites Check:"
echo "-------------------"

# Check Node.js
if command -v node &> /dev/null; then
    NODE_VERSION=$(node -v)
    echo "✓ Node.js: $NODE_VERSION"
else
    echo "✗ Node.js not found. Please install Node.js 18+"
    exit 1
fi

# Check Rust
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    echo "✓ Rust: $RUST_VERSION"
else
    echo "✗ Rust not found. Please install Rust 1.77+"
    exit 1
fi

# Check if dependencies are installed
if [ -d "node_modules" ]; then
    echo "✓ Node dependencies installed"
else
    echo "✗ Node dependencies not found. Run: npm install"
    exit 1
fi

echo ""
echo "=========================================="
echo "Starting AutoCorrect Development Server"
echo "=========================================="
echo ""
echo "The application will launch in development mode."
echo "Please follow the testing steps below:"
echo ""

cat << 'EOF'
┌─────────────────────────────────────────────────────────────────┐
│  TESTING GUIDE                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. BASIC SPELL CHECKING                                        │
│     - When the app opens, you'll see the main Spell Check tab  │
│     - Type some text with errors (e.g., "heloo world")         │
│     - Click "Check Spelling" button                            │
│     - Verify corrections are suggested                         │
│                                                                 │
│  2. GLOBAL HOTKEY TEST (Cmd+Shift+A or Ctrl+Shift+A)           │
│     - Open any text editor (TextEdit, VS Code, etc.)           │
│     - Type text with spelling errors                           │
│     - Select the text                                          │
│     - Press Cmd+Shift+A (macOS) or Ctrl+Shift+A (Win/Linux)   │
│     - A popup should appear with corrections                   │
│     - Press Enter to accept or Esc to dismiss                  │
│                                                                 │
│  3. SETTINGS PANEL                                             │
│     - Click the "Settings" tab in the top navigation           │
│     - Test toggling "Enable AutoCorrect" on/off                │
│     - Verify the status indicator updates                      │
│     - Check "Enable Clipboard Monitor" option                  │
│     - Try adjusting the "CJK Only" and "Poll Interval"        │
│     - Click "Save Configuration"                               │
│                                                                 │
│  4. CLIPBOARD MONITORING TEST                                  │
│     - In Settings, enable "Clipboard Monitor"                  │
│     - Open a text editor and type: "testt texxt"               │
│     - Select and copy the text (Cmd+C or Ctrl+C)               │
│     - The app should detect it automatically                  │
│     - Try pasting (Cmd+V) to see if corrections were applied  │
│                                                                 │
│  5. STATUS INDICATOR                                           │
│     - Look at the top-right corner of the app                 │
│     - Verify it shows "Enabled" with a green dot              │
│     - Toggle it off/on from Settings                           │
│     - Verify the status updates correctly                     │
│                                                                 │
│  6. ABOUT PAGE                                                  │
│     - Click the "About" tab                                    │
│     - Verify the app information displays correctly            │
│     - Check that version shows 0.1.0                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

EOF

echo "Press Enter to start the application..."
read

# Start the dev server
echo ""
echo "Starting development server..."
echo "Note: First launch may take longer as Rust compiles the backend..."
echo ""

npm run tauri:dev

echo ""
echo "=========================================="
echo "Demo session ended"
echo "=========================================="
echo ""
echo "Next steps:"
echo "- Run 'npm run tauri:build' to create a release build"
echo "- See README.md for full documentation"
echo "- Report issues at: https://github.com/huacnlee/autocorrect/issues"
