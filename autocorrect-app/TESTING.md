# AutoCorrect Desktop App - Testing Guide

This guide provides step-by-step instructions for testing all features of the AutoCorrect desktop application.

## Prerequisites

Before testing, ensure you have:

- Node.js 18+ installed
- Rust 1.77+ installed
- Application dependencies installed: `npm install`
- Development server running: `npm run tauri:dev`

### macOS Permission Setup (required before testing)

The overlay and hotkey features require macOS permissions. In **development mode** (`tauri:dev`) these are requested from the terminal process. For a **packaged build** follow [README.md — macOS Permissions Setup](./README.md#macos-permissions-setup).

For dev mode, grant Accessibility permission to your terminal app (Ghostty, iTerm2, Terminal.app) in **System Settings → Privacy & Security → Accessibility**.

## Quick Start

1. Start the application: `npm run tauri:dev`
2. The app window will open automatically
3. Follow the test cases below

## Test Cases

### 1. Overlay Red Underlines (Hover Mode)

**Objective**: Verify real-time typo markers appear under misspelled words.

**Steps**:
1. Open **Notes**, **TextEdit**, or **Slack** (not a terminal emulator)
2. Click into a text input area and type: `teh quick brwon fox`
3. Wait ~1 second
4. Verify red underlines appear under `teh` and `brwon`

**Expected Result**:
- Red underline markers appear directly under each typo
- Correct words have no underline
- Markers update within ~1 s of typing

---

### 2. Hover Popup and Keyboard Acceptance

**Objective**: Verify the suggestion popup appears on hover and accepts via keyboard.

**Steps**:
1. With overlay markers visible (see Test 1), hover the mouse over `teh`
2. Verify a popup window appears near the cursor with the suggestion `the`
3. Press `Enter`
4. Verify `teh` is replaced by `the` in the source app, and the popup closes

**Expected Result**:
- Popup appears within ~300 ms of hovering
- Popup receives keyboard focus automatically (title bar active)
- `Enter` replaces the typo word; source app retains focus
- `Esc` dismisses the popup without changes

---

### 3. Hover Popup Click Acceptance

**Objective**: Verify clicking the Accept button also replaces the typo.

**Steps**:
1. Hover over another typo underline (e.g., `brwon`) to show popup
2. Click the **Accept** button in the popup
3. Verify `brwon` is replaced by `brown` in the source app

**Expected Result**:
- Click on Accept replaces the exact typo word
- Text before and after the typo is unchanged
- Popup closes after acceptance

---

### 4. Basic Spell Checking

**Objective**: Verify the core spell checking functionality works correctly.

**Steps**:
1. With the app open, you should see the "Spell Check" tab as the default view
2. In the text input area, type: `hello world this is a testt`
3. Click the "Check Spelling" button
4. Verify that corrections are suggested (e.g., "testt" → "test")
5. Click "Accept" to apply the correction
6. Verify the text is updated in the input area

**Expected Result**:
- Corrections are displayed
- Text is updated when accepting suggestions
- No errors in the console

---

### 2. Global Hotkey

**Objective**: Test the global hotkey functionality for in-place correction.

**Steps**:
1. Open a text editor (TextEdit, Notes, VS Code, etc.)
2. Type text with errors: `Thiss is a testt of the hotkey featurre`
3. Select the text you just typed
4. Press the global hotkey:
   - **macOS**: `Cmd + Shift + A`
   - **Windows/Linux**: `Ctrl + Shift + A`
5. A popup should appear near your selection with corrections
6. Press `Enter` to accept or `Esc` to dismiss

**Expected Result**:
- Popup appears with corrections
- Pressing Enter applies corrections to the selected text
- Pressing Esc dismisses the popup without changes
- Hotkey works from any application

**Troubleshooting**:
- If hotkey doesn't work, check system permissions:
  - **macOS**: System Settings → Privacy & Security → Accessibility
  - **Windows**: Ensure AutoCorrect has input monitoring permissions

---

### 3. Settings Panel

**Objective**: Verify settings can be viewed and modified.

**Steps**:
1. Click the "Settings" tab in the top navigation
2. Verify the following elements are visible:
   - Enable/Disable AutoCorrect toggle
   - Hotkey configuration section
   - Clipboard Monitor section
   - Correction Rules section
3. Toggle "Enable AutoCorrect" off
4. Verify the status indicator in the top-right changes to "Disabled"
5. Toggle it back on
6. Check "Enable Clipboard Monitor"
7. Adjust "CJK Only" and "Poll Interval" settings
8. Click "Save Configuration"

**Expected Result**:
- All settings are visible and editable
- Status indicator updates when toggling AutoCorrect
- Settings persist after saving
- Success message appears after saving

---

### 4. Clipboard Monitoring

**Objective**: Test automatic clipboard text correction.

**Steps**:
1. Go to Settings tab
2. Enable "Clipboard Monitor"
3. Set "CJK Only" to false (to test with English text)
4. Set "Poll Interval" to 500ms
5. Click "Save Configuration"
6. Open a text editor
7. Type: `Thiss text willl be corrected automaticlly`
8. Select and copy the text (Cmd+C or Ctrl+C)
9. Wait a moment (within the poll interval)
10. Check the AutoCorrect app for a notification or log
11. Paste the text (Cmd+V or Ctrl+V)

**Expected Result**:
- App detects when clipboard changes
- Corrections are applied automatically
- Pasting yields corrected text

**Note**: Full clipboard replacement is a planned feature. Currently, the app detects clipboard changes but may not replace content automatically.

---

### 5. Status Indicator

**Objective**: Verify the status indicator shows correct state.

**Steps**:
1. Look at the top-right corner of the app window
2. Verify it shows:
   - Green dot with "Enabled" text when AutoCorrect is on
   - Gray/red dot with "Disabled" text when AutoCorrect is off
3. Go to Settings and toggle AutoCorrect off
4. Return to main tab and verify the indicator changed
5. Toggle back on and verify it changes again

**Expected Result**:
- Status indicator reflects the current state
- Updates immediately when toggled
- Visual distinction between enabled/disabled states

---

### 6. About Page

**Objective**: Verify the About page displays correctly.

**Steps**:
1. Click the "About" tab in the top navigation
2. Verify the following information:
   - AutoCorrect logo/icon
   - App name: "AutoCorrect"
   - Description text
   - Feature list
   - Version number (0.1.0)
   - Credits (Tauri + Svelte)

**Expected Result**:
- All information is displayed correctly
- Layout is clean and readable
- No broken images or missing text

---

### 7. Multi-language Support

**Objective**: Test spell checking with CJK characters.

**Steps**:
1. Go to the main Spell Check tab
2. Type or paste Chinese text: `你好世界`
3. Click "Check Spelling"
4. Type mixed content: `Hello 你好 world`
5. Click "Check Spelling"

**Expected Result**:
- CJK text is preserved correctly
- Mixed content is handled properly
- No encoding issues

---

### 8. Configuration Persistence

**Objective**: Verify settings persist across app restarts.

**Steps**:
1. Go to Settings
2. Change several settings:
   - Disable AutoCorrect
   - Enable Clipboard Monitor
   - Change Poll Interval to 1000ms
3. Click "Save Configuration"
4. Completely quit the app
5. Restart the app: `npm run tauri:dev`
6. Go to Settings

**Expected Result**:
- All settings are restored to their saved values
- No settings are lost after restart

---

### 9. Performance Test

**Objective**: Ensure the app handles large texts efficiently.

**Steps**:
1. Go to the main Spell Check tab
2. Paste a large block of text (several paragraphs)
3. Click "Check Spelling"
4. Monitor the response time

**Expected Result**:
- Spell checking completes in under 2 seconds for typical texts
- UI remains responsive during processing
- No crashes or hangs

---

### 10. Error Handling

**Objective**: Verify the app handles errors gracefully.

**Steps**:
1. Try checking empty text
2. Try checking text with only special characters
3. Toggle settings rapidly
4. Try to save invalid configuration values

**Expected Result**:
- App doesn't crash on invalid input
- Helpful error messages are displayed
- App recovers gracefully from errors

---

## Test Results Template

Use this template to track your test results:

| Test Case | Status | Notes |
|-----------|--------|-------|
| Overlay Red Underlines | ☐ Pass / ☐ Fail | |
| Hover Popup (keyboard) | ☐ Pass / ☐ Fail | |
| Hover Popup (click) | ☐ Pass / ☐ Fail | |
| Basic Spell Checking | ☐ Pass / ☐ Fail | |
| Global Hotkey | ☐ Pass / ☐ Fail | |
| Settings Panel | ☐ Pass / ☐ Fail | |
| Clipboard Monitoring | ☐ Pass / ☐ Fail | |
| Status Indicator | ☐ Pass / ☐ Fail | |
| Multi-language Support | ☐ Pass / ☐ Fail | |
| Configuration Persistence | ☐ Pass / ☐ Fail | |
| Performance Test | ☐ Pass / ☐ Fail | |
| Error Handling | ☐ Pass / ☐ Fail | |

## Known Issues / Limitations

- **Terminal emulators** (Ghostty, iTerm2, Terminal.app, VS Code terminal) are intentionally excluded from overlay monitoring
- **Overlay bounds** rely on the macOS Accessibility API; apps that do not expose `AXBoundsForRange` fall back to estimated positions which may be slightly off
- Accessibility permission must be re-granted after each `pnpm tauri build` for unsigned builds (see README for the permanent fix via code signing)
- Global hotkey may conflict with other applications using `Cmd+Shift+A`

## Reporting Issues

If you find a bug or unexpected behavior:

1. Check the [Known Issues](#known-issues) section
2. Search existing [GitHub Issues](https://github.com/huacnlee/autocorrect/issues)
3. Create a new issue with:
   - Operating system and version
   - Steps to reproduce
   - Expected vs actual behavior
   - Console logs (if any)

## Next Steps After Testing

- Run `npm run tauri:build` to create a release build
- Test the release build (not dev mode)
- Provide feedback on the GitHub repository
- Contribute improvements if you find issues
