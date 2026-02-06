# Testing Typos Display - Quick Guide

## What Was Fixed

The SpellChecker component and popup window now properly display typos detected by CSpell and the typos library. Previously, only CJK formatting corrections (`line_changes`) were displayed, but spelling errors (`typos`) were missing from the UI.

## Changes Made

### 1. SpellChecker.svelte
- ✅ Added `typos` array extraction from spell_check result
- ✅ Added typos display section with yellow warning boxes
- ✅ Added `applyTypoSuggestion()` function to replace typos with suggestions
- ✅ Added `addToCustomCorrections()` function to save typo corrections
- ✅ Added console.log for debugging

### 2. Frontend Rebuilt
- ✅ Ran `pnpm run build` to compile Svelte components
- ✅ Updated dist/ with new code

## How to Test

### Step 1: Start the Application

```bash
cd autocorrect-app
pnpm tauri dev
```

**Wait for:**
- Rust compilation to finish
- App window to open

### Step 2: Test Main Window SpellChecker

1. **Click "Spell Checker" tab** in the main window
2. **Type test text with clear typos:**
   ```
   naem funciton areyour
   ```
3. **Click "Check Spelling" button**

**Expected Result:**
- ✅ Yellow box appears with heading "Spelling Issues (3)"
- ✅ Each typo shows:
  - Typo word in red quotes: "naem"
  - Location: "Line 1, Column 1"
  - Green suggestion buttons: "name", "nam", etc.
  - Blue button: "Add 'naem' → 'name' to Custom Corrections"

4. **Click a green suggestion button**

**Expected Result:**
- ✅ Typo is replaced in the text area
- ✅ Spell check runs again automatically
- ✅ Typo disappears from the list

5. **Click "Add to Custom Corrections" button**

**Expected Result:**
- ✅ Typo is removed from list
- ✅ Added to `~/.autocorrect-typos.txt`

### Step 3: Test Popup Window

1. **Type test text in any text editor** (TextEdit, Notes, etc.):
   ```
   naem funciton
   ```

2. **Select the text**

3. **Press hotkey: Cmd+Shift+K** (macOS) or **Ctrl+Shift+K** (Linux/Windows)

**Expected Result:**
- ✅ Popup window appears
- ✅ Shows "Original" and "Suggestion" (may be same if no CJK corrections)
- ✅ Yellow "Spelling Issues" section appears below
- ✅ Shows both typos with suggestions

4. **Click a suggestion button in popup**

**Expected Result:**
- ✅ Typo is replaced in the suggestion text
- ✅ Typo disappears from the list

5. **Click "Accept" button**

**Expected Result:**
- ✅ Corrected text is pasted back into editor
- ✅ Popup closes

### Step 4: Open Browser DevTools (for debugging)

**If popup doesn't show typos:**

1. **Right-click on popup window** → **Inspect Element**
2. **Open Console tab**
3. **Trigger popup again** (Cmd+Shift+K with text selected)

**Look for these logs:**
```
Popup show: {originalText: "naem funciton", suggestion: "naem funciton", ...}
Typos received: [{typo: "naem", suggestions: ["name"], line: 1, col: 1}, ...]
Typos type: object
Typos is array? true
About to render typos list with: [...]
renderTyposList called with: [...]
typos length: 2
Showing typos section for 2 typos
Rendering typo 0: {typo: "naem", ...}
Rendering typo 1: {typo: "funciton", ...}
Finished rendering typos, typosListEl children: 2
```

**If logs show data but UI doesn't render:**
- Check for CSS issues: `typosEl.classList` should NOT contain 'hidden'
- Check `typosListEl.innerHTML` is not empty

## Test Cases

### ✅ Valid Typos (Should be detected)

| Input | Detected | Suggestions |
|-------|----------|-------------|
| `naem` | ✅ | name, nam, named |
| `funciton` | ✅ | function, functions, functioned |
| `areyour` | ✅ | are your, are you, are yours |
| `teh` | ✅ | the, tea, tech |
| `recieve` | ✅ | receive, received |

### ❌ Not Detected (Use Custom Corrections)

| Input | Why Not Detected |
|-------|------------------|
| `whats` | CSpell considers it valid informal English |
| `hows` | CSpell considers it valid informal English |
| `gonna` | Valid colloquial word |

**To add these:**
1. Go to "Custom Corrections" tab
2. Add: `whats=what's`
3. Add: `hows=how's`
4. Test again - should now be corrected

## Configuration Check

### Verify CSpell is Enabled

```bash
cat ~/.autocorrect-app.json | grep cspell_enabled
```

**Expected:** `"cspell_enabled": true`

**If false:**
1. Open app → Settings tab
2. Enable "CSpell Programming Dictionaries"
3. Enable desired dictionaries (TypeScript, JavaScript, etc.)
4. Click "Save Settings"

### Verify CSpell CLI Exists

```bash
ls -la autocorrect-app/node_modules/.bin/cspell
```

**Expected:** File exists and is executable

**If missing:**
```bash
cd autocorrect-app
pnpm install
```

## Troubleshooting

### Issue: No typos appear in main window

**Check:**
1. Is CSpell enabled in Settings?
2. Are you using valid test typos (see "Valid Typos" table)?
3. Check browser DevTools console for errors

**Fix:**
- Stop app (Ctrl+C)
- Rebuild: `pnpm run build`
- Restart: `pnpm tauri dev`

### Issue: Popup doesn't show typos

**Check:**
1. Open DevTools on popup (right-click → Inspect)
2. Look for console logs (see Step 4 above)
3. Check if data is received but not rendered

**Debug:**
- Look for "typos received" log
- Check if array is empty or populated
- Verify `renderTyposList()` is called

### Issue: CSpell not detecting anything

**Check:**
```bash
# Test CSpell CLI directly
cd autocorrect-app
echo "naem funciton" | pnpm cspell stdin --no-progress
```

**Expected output:**
```
/dev/stdin:1:1 - Unknown word (naem)
/dev/stdin:1:6 - Unknown word (funciton)
```

**If no output:**
- CSpell CLI not installed: `pnpm install`
- CSpell config issue: Check `cspell.config.js`

### Issue: Rust backend errors

**Check logs:**
```bash
RUST_LOG=debug pnpm tauri dev
```

**Look for:**
- `INFO - Found CSpell at: ...` (path found)
- `DEBUG - CSpell result: ...` (detection working)
- `ERROR` messages (problems)

## Success Criteria

✅ **Main Window:**
- Yellow "Spelling Issues" box appears
- Shows all typos with line/column numbers
- Green suggestion buttons work
- Blue "Add to Custom" button works

✅ **Popup Window:**
- Yellow typos section appears below suggestion
- Shows all typos with suggestions
- Clicking suggestion updates text
- Accept button pastes corrected text

✅ **Custom Corrections:**
- Can add typos to custom corrections
- Custom corrections work immediately (no restart)
- File `~/.autocorrect-typos.txt` updated

## Next Steps After Testing

If everything works:
1. ✅ Mark this issue as resolved
2. 📝 Document CSpell feature in user guide
3. 🎨 Consider UI polish (animations, better styling)
4. 🔄 Add "Fix All" button to apply all suggestions at once

If something doesn't work:
1. Share console logs from DevTools
2. Share output of `cat ~/.autocorrect-app.json`
3. Share result of `echo "naem" | pnpm cspell stdin`
4. Share any Rust error logs (RUST_LOG=debug output)

## Files Modified

- `src/lib/components/SpellChecker.svelte` - Added typos display
- `dist/` - Rebuilt frontend assets
- This guide: `TESTING_TYPOS_DISPLAY.md`

**No changes needed to:**
- `src/popup.ts` - Already had typos rendering (just added debug logs)
- `src-tauri/src/cspell.rs` - Already working correctly
- `src-tauri/src/commands/spellcheck.rs` - Already returning typos

## Estimated Test Time

- ⏱️ Full testing: 10-15 minutes
- ⏱️ Quick smoke test: 3-5 minutes

Happy testing! 🎉
