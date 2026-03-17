# Research Report: Getting Exact Screen Position of Characters in Third-Party App Text Inputs on macOS

Generated: 2026-02-21

## Executive Summary

Getting the exact screen coordinates of specific characters in third-party app text fields on macOS is fundamentally difficult, especially for Electron apps (like Slack) where `AXBoundsForRange` returns zero-sized rects. After extensive research, the most viable approach for your use case is a **hybrid strategy**: use `AXBoundsForRange` where it works (native apps), and fall back to **Screen OCR with Vision framework** for Electron/Chromium apps where the Accessibility API fails. Grammarly itself does NOT draw underlines on arbitrary third-party desktop apps -- it uses browser extensions, Office add-ins, and a floating widget positioned near the text field (not at per-character precision).

## Research Question

How to get the exact screen position of specific characters/words in a third-party app's text input on macOS, specifically when `AXBoundsForRange` fails for Electron apps.

---

## Key Findings

### Finding 1: AXBoundsForRange Limitations with Electron/Chromium

**The core problem**: Electron apps use Chromium's accessibility implementation, which exposes `AXValue` (the text content) but does NOT properly implement `AXBoundsForRange` for parameterized attribute queries. This is a known limitation of Chromium's accessibility bridge on macOS.

**What works in Electron apps via Accessibility API**:

- `AXValue` -- full text content of the focused element (confirmed working in your codebase)
- `AXSelectedText` -- currently selected text
- `AXSelectedTextRange` -- range of the selection
- `AXRole` -- the element's role
- `AXFocusedUIElement` -- getting the focused element itself
- `AXPosition` -- the position of the entire UI element (the text field)
- `AXSize` -- the size of the entire UI element

**What does NOT work reliably in Electron**:

- `AXBoundsForRange` -- returns zero-sized rect (your current problem)
- `AXRangeForPosition` -- may not work in Electron
- `AXLineForIndex` / `AXRangeForLine` -- inconsistent support

**Possible workaround within AX API**: You CAN get the bounds of the entire text field via `AXPosition` + `AXSize`. Combined with knowing the text content and the caret position, you can estimate character positions -- but this requires knowing the font metrics, which are not exposed by the AX API for Electron apps.

- Source: macOS Accessibility API documentation, Electron accessibility docs, StackOverflow discussion at stackoverflow.com/questions/9336981

### Finding 2: How Grammarly Actually Works on macOS

**Critical insight**: Grammarly does NOT overlay precise per-character underlines on arbitrary third-party desktop apps. Grammarly's approach is fundamentally different from what you are trying to build:

1. **Browser extension**: For web apps (including Slack web, Gmail, etc.), Grammarly injects JavaScript via a browser extension. It directly manipulates the DOM to add underlines. This is how it achieves pixel-perfect underlines in browsers.

2. **Office add-in**: For Microsoft Word, Grammarly uses the Word add-in API. It temporarily applies formatting (thick red underline via `Range.Font.Underline`) to mark errors. Before save/print events (`Document.BeforeSave`, `Document.BeforePrint`), it removes the formatting to avoid persisting it. This was confirmed by a StackOverflow answer analyzing Grammarly's Word plugin behavior.

3. **macOS desktop app / floating widget**: The Grammarly for Mac desktop app works by showing a **small floating circle/widget** near the bottom-right corner of the active text field. It does NOT draw underlines directly on the text. When you click the widget, it opens a panel showing suggestions. The widget is positioned relative to the entire text field bounds (using `AXPosition` + `AXSize` of the focused element), NOT at per-character precision.

4. **For Electron apps like Slack desktop**: Grammarly's desktop app shows the floating widget near the text input area, but it does NOT underline individual words. Users who want underlines in Slack must use the browser version of Slack with the Grammarly browser extension.

**Implication for your project**: Achieving Grammarly-like per-character underlines in Electron apps from an external process is something even Grammarly has NOT solved. Their solution is to use browser extensions for web-based apps, not overlay windows.

- Source: Grammarly official site (grammarly.com/native/mac), StackOverflow analysis of Grammarly Word plugin behavior, user reports about Grammarly behavior in various apps

### Finding 3: Screen OCR Approach (Vision Framework) -- Most Viable Alternative

**This is the most promising approach for Electron apps.** A recent real-world project (SnapTra Translator, January 2026) demonstrates a complete working pipeline:

**Pipeline**:

1. Capture a screen region around the focused text field using `ScreenCaptureKit` (or `CGWindowListCreateImage`)
2. Feed the captured image to `VNRecognizeTextRequest` from Apple's Vision framework
3. Vision returns recognized text with **bounding boxes** for each text line/word
4. Map the recognized words to your typo list to find exact screen positions

**Key technical details**:

- Vision returns bounding boxes in **normalized coordinates** (0.0 to 1.0), which you convert to screen coordinates using the capture region's screen rect
- `recognitionLevel = .accurate` gives best results for text position accuracy
- The y-axis is flipped (Vision uses bottom-left origin, screen uses top-left)
- Requires Screen Recording permission (`CGPreflightScreenCaptureAccess()`)
- On macOS 13+, `automaticallyDetectsLanguage` handles CJK + English mixed text
- For CJK text, word-level bounding boxes may need custom splitting since Vision returns line-level results

**Performance considerations**:

- Capture only a small region around the text field (not full screen) -- the SnapTra project uses 520x140px
- OCR a region this size takes ~50-100ms on Apple Silicon
- With an 800ms polling interval (matching your current `sync_system_typos` approach), this is feasible

**Limitations**:

- Requires Screen Recording permission (in addition to Accessibility)
- OCR results may not be pixel-perfect -- typically within 2-5px accuracy
- Performance overhead is higher than Accessibility API
- Font rendering differences between the app and OCR can cause minor misalignment
- Does not work if text is obscured or offscreen

- Source: SnapTra Translator implementation (juejin.cn/post/7598435950411104266), Apple Vision framework documentation, Screen2AX research paper (arxiv.org/html/2507.16704v1)

### Finding 4: Input Method Kit (IMK) Approach

**Concept**: Create a custom macOS input method that intercepts text input and can annotate it.

**How it works**:

- An IMK-based input method receives every keystroke via `inputText:client:` callback
- The `client` parameter gives you an `id` representing the text input field
- You can call `setMarkedText:selectionRange:replacementRange:` to show inline annotations (underlined text in the composition buffer)
- The `mark(forStyle:at:)` method controls the appearance of marked text

**Why this is NOT suitable**:

1. **Users must actively select your input method**: It replaces their normal keyboard input. You cannot run it alongside their preferred input method (e.g., Chinese Pinyin).
2. **It only works during active input**: You cannot annotate existing text that was already typed.
3. **Marked text is temporary**: It disappears once committed. You cannot persistently underline a word.
4. **Conflict with existing input methods**: CJK users already use IMEs, and macOS allows only one active input source at a time.
5. **Installation complexity**: Input methods must be placed in `/Library/Input Methods/` and require special bundle identifier formatting.

**Conclusion**: IMK is completely unsuitable for a Grammarly-like overlay tool. It is designed for text input, not text annotation.

- Source: Apple InputMethodKit documentation (developer.apple.com/documentation/inputmethodkit), Multiple IMK development guides (logcg.com, blog.csdn.net/dinjay)

### Finding 5: CGEvent / AXTextMarker Approach

**AXTextMarker / AXTextMarkerRange**: These are private, undocumented accessibility attributes used internally by Apple's VoiceOver. They represent opaque position markers within text content.

**Reality**:

- `AXTextMarker` and `AXTextMarkerRange` are NOT part of the public Accessibility API
- They are used internally by VoiceOver and are specific to certain Apple apps (Safari, TextEdit)
- Chromium/Electron does NOT implement these private markers
- Using private API risks App Store rejection and breakage across macOS versions
- There is no documentation on how to convert `AXTextMarker` positions to screen coordinates

**CGEvent approach**: Using `CGEventTap` to intercept keyboard events gives you no information about text layout or character positions. It only provides keycode/character data.

**Conclusion**: This approach is a dead end for Electron apps.

- Source: macOS Accessibility API headers, accessibility developer forums

### Finding 6: Hybrid Approach Used by Similar Tools

**LanguageTool Desktop**: Uses a similar approach to Grammarly -- floating widget near the text field, not per-character overlays. Falls back to clipboard-based text extraction.

**Bob (macOS translation tool)**: Uses `AXSelectedText` + `AXBoundsForRange` for the selected text range only, not for arbitrary ranges. When `AXBoundsForRange` fails, it uses the mouse cursor position as an approximation.

**Eudic / PopClip**: These tools show a popup near the mouse cursor position or near the text selection bounds, rather than at exact character positions.

**SnapTra Translator**: Uses Screen OCR (Vision framework) to find word positions, as described in Finding 3.

**macOS Hover Text (built-in accessibility)**: Apple's own "Hover Text" feature (`mchlb203bc78`) uses the Accessibility API and Vision framework together. It captures the area under the cursor and uses OCR to enlarge and display text -- notably, it works system-wide including in Electron apps.

- Source: Various tool documentation and community discussions

---

## Codebase Analysis

Your current implementation in `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/macos_text.rs`:

- Uses `AXBoundsForRange` (line 104-109) to get screen coordinates for text ranges
- Falls back to returning a default (zero) `CGRect` when this fails (line 122)
- Has a `get_focused_text_context()` function that successfully extracts text content from Electron apps via `AXValue`
- Already handles UTF-16 range calculations for the Accessibility API

Your overlay system in `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/overlay.rs`:

- Creates native macOS overlay windows
- Positions `TypoMarker` elements at specific (x, y, width, height) coordinates
- Already has the rendering infrastructure in place

The gap is specifically: when `AXBoundsForRange` returns zero, you have no character positions to feed to the overlay.

---

## Recommendations

### Recommended Strategy: Tiered Fallback System

**Tier 1: AXBoundsForRange (current approach)**

- Keep using this for native macOS apps (TextEdit, Notes, Pages, Xcode, etc.) where it works perfectly
- Detection: If `AXBoundsForRange` returns a non-zero rect, use it

**Tier 2: Screen OCR via Vision Framework (new, for Electron apps)**

- When `AXBoundsForRange` returns zero, capture the text field region and run OCR
- Steps:
  1. Get the text field's overall bounds via `AXPosition` + `AXSize` (this DOES work for Electron)
  2. Capture that screen region using `CGWindowListCreateImage` or `ScreenCaptureKit`
  3. Run `VNRecognizeTextRequest` on the captured image
  4. Match OCR results against your typo list to find bounding boxes
  5. Convert Vision's normalized coordinates to screen coordinates
- Requires Screen Recording permission
- OCR can be cached and only re-run when text changes (detected via `AXValue` polling)

**Tier 3: Approximate positioning (fallback for when OCR is not available)**

- Position the popup/overlay relative to the entire text field, not at character precision
- Use a floating widget approach similar to Grammarly's desktop app
- This is a degraded but still useful experience

### Why NOT to pursue other approaches:

- **IMK (Input Method)**: Fundamentally wrong architecture; conflicts with CJK input methods
- **AXTextMarker**: Private API, not implemented by Electron
- **CGEvent**: No layout information available
- **Browser extension injection**: Would only work if you ship a companion browser extension, not from a Tauri app

---

## Open Questions

1. **Screen Recording permission UX**: Adding another permission request (on top of Accessibility) may frustrate users. Need to consider whether this is acceptable.

2. **OCR accuracy for CJK text**: Vision framework handles CJK well, but mixed CJK/English text with small font sizes may produce less accurate bounding boxes. Needs testing.

3. **Performance with continuous OCR**: If running OCR every 800ms polling cycle, what is the CPU/GPU impact? May need to throttle OCR or only run it when text changes are detected.

4. **Retina display scaling**: Screen capture coordinates need proper DPI/scale factor handling. The current overlay code already handles `screen_height` and frame origin, but OCR coordinate conversion adds another layer.

5. **Multi-line text fields**: When the text field has scrollable content, characters may be offscreen. OCR would only see visible text, which may actually be desirable (only overlay visible errors).

6. **Alternative: Could you inject into Electron's renderer process?** Theoretically, if you could communicate with Slack's renderer process (e.g., via Chrome DevTools Protocol on the debug port), you could use `getBoundingClientRect()` in JavaScript to get exact positions. This would require Slack to be launched with remote debugging enabled, which is not practical for end users.
