# AutoCorrect App - Crash Debug Session

## RESOLVED ✓

The app crash on startup has been fixed. The issue was caused by:

1. **rdev macOS crash bug** - The upstream rdev crate has a known issue on macOS that causes crashes when listening to keyboard events
2. **Channel shadowing bug** - lib.rs had two `hotkey_rx` variables where the thread was listening on the wrong channel

## Solution Applied

### 1. Fixed rdev dependency (Cargo.toml)

```toml
# Use forked rdev that fixes macOS crash issue (see Tauri discussion #7839)
rdev = { git = "https://github.com/fufesou/rdev" }
```

This fork is maintained by the RustDesk team and fixes the `dispatch_assert_queue_fail` crash that occurs in the original rdev crate when calling `rdev::listen` on macOS.

Reference: https://github.com/tauri-apps/tauri/discussions/7839

### 2. Fixed channel shadowing bug (lib.rs)

- Removed unused channel creation on line 27
- Changed `(_hotkey_rx, ...)` to `(hotkey_rx, ...)` on line 45 to properly capture the receiver

## Current State

- App starts successfully without crashes
- Hotkey listener initializes with Cmd+Shift+A
- Vite dev server running on http://localhost:1420/
- All Tauri commands registered and functional

## Files Modified

- `src-tauri/Cargo.toml` - Updated rdev to use fixed fork
- `src-tauri/src/lib.rs` - Fixed channel variable shadowing
