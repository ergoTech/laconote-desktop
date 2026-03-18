# Technical Specification: Fix System Audio Permission Detection on macOS 15+

## Technical Context

- **Language**: Rust (backend), TypeScript/React (frontend), Objective-C (native bridge)
- **Framework**: Tauri v2
- **Target**: macOS 14.2+ (CATap API), focus on macOS 15+ (Sequoia) where Apple split permissions
- **Key dependencies**: CoreAudio framework (`AudioHardwareCreateProcessTap`), CoreGraphics (`CGPreflightScreenCaptureAccess`)

## Problem Summary

`check_system_audio()` uses `CGPreflightScreenCaptureAccess()` which only detects the full "Screen Recording" permission. On macOS 15+, the separate "System Audio Recording Only" permission is invisible to this API. The field `system_audio_capture_ready` is hardcoded to `Unknown`, so the derived status is always `Unknown` when the user only has audio-only permission.

## Implementation Approach

### Change 1: Lightweight CATap permission probe (Objective-C bridge)

**File**: `src-tauri/src/audio/cat_tap_bridge.m`

Add a new C function `catap_probe_permission()` that:
1. Creates a `CATapDescription` (same as `catap_start`)
2. Calls `AudioHardwareCreateProcessTap`
3. If `noErr` → immediately calls `AudioHardwareDestroyProcessTap` → returns `1`
4. If error → returns `0`

This function does NOT create an IOProc, does NOT start audio device I/O, does NOT register callbacks. It only tests whether the tap can be created (which requires either Screen Recording or System Audio Recording Only permission).

Expected latency: < 50ms (single CoreAudio IPC call).

```c
int catap_probe_permission(void) {
    if (!isAtLeastMacOS14_2()) return 0;
    __block int result = 0;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            CATapDescription *desc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
            desc.name = @"LaconotePermissionProbe";
            desc.privateTap = YES;
            desc.muteBehavior = CATapUnmuted;
            AudioObjectID tapID = kAudioObjectUnknown;
            OSStatus status = AudioHardwareCreateProcessTap(desc, &tapID);
            if (status == noErr) {
                AudioHardwareDestroyProcessTap(tapID);
                result = 1;
            }
        }
    });
    return result;
}
```

**Concern**: This shares `gTapQueue` with `catap_start`/`catap_stop`. If a recording is active, `dispatch_sync` on the serial queue will block until the current block finishes. This is acceptable because:
- Permission checks don't happen during recording
- Even if they did, the queue operations are sub-millisecond

### Change 2: Expose probe via Rust FFI

**File**: `src-tauri/src/audio/cat_tap.rs`

Add FFI declaration and wrapper:
```rust
extern "C" {
    fn catap_probe_permission() -> i32;
}

pub fn probe_permission() -> bool {
    unsafe { catap_probe_permission() != 0 }
}
```

**File**: `src-tauri/src/audio/mod.rs`

Re-export:
```rust
pub use cat_tap::probe_permission as probe_catap_permission;
```

### Change 3: Use probe in `check_system_audio()`

**File**: `src-tauri/src/permissions.rs`

Current flow:
```
CGPreflightScreenCaptureAccess() → true  → screen_capture_access = Granted
                                → false → screen_capture_access = NotGranted
system_audio_capture_ready = Unknown (always)
derive_system_audio_status(screen_capture_access, Unknown) → Unknown
```

New flow:
```
CGPreflightScreenCaptureAccess() → true  → screen_capture_access = Granted
                                         → system_audio_capture_ready = Granted
                                → false → screen_capture_access = NotGranted
                                         → probe_permission()
                                           → true  → system_audio_capture_ready = Granted
                                           → false → system_audio_capture_ready = NotGranted
derive_system_audio_status(screen_capture_access, system_audio_capture_ready) → accurate result
```

When `CGPreflightScreenCaptureAccess()` returns `true`, we skip the probe — the full Screen Recording permission implies audio access. The probe is only the fallback for the macOS 15+ audio-only permission case.

### Change 4: Update `derive_system_audio_status()`

The current function already handles the new states correctly:
- `(NotGranted, Granted)` → `Granted` ✓ (audio-only permission on macOS 15+)
- `(Granted, Granted)` → `Granted` ✓ (full screen recording)
- `(NotGranted, NotGranted)` → `NotGranted` ✓ (no permission)
- `(*, Unknown)` → `Unknown` ✓ (should not happen anymore, but safe fallback)

No changes needed to this function. Existing tests remain valid.

### Change 5: Add `Privacy_ListenEvent` settings URL for macOS 15+

**File**: `src-tauri/src/permissions.rs`

The current `resolve_system_settings_urls("system_audio")` points to `Privacy_ScreenCapture`, which on macOS 15+ opens "Screen & System Audio Recording" — the wrong pane. The correct pane for audio-only is `Privacy_ListenEvent`.

Update the URL list to prioritize `Privacy_ListenEvent` (system audio only) before `Privacy_ScreenCapture` (full screen recording):

```rust
"system_audio" => &[
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_ListenEvent",
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_AudioCapture",
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_ScreenCapture",
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension",
    "x-apple.systempreferences:com.apple.preference.security?Privacy",
    "x-apple.systempreferences:com.apple.preference.security",
],
```

- `Privacy_ListenEvent` — "System Audio Recording Only" pane (macOS 15+)
- `Privacy_AudioCapture` — alternative audio capture pane (macOS 15+)
- `Privacy_ScreenCapture` — fallback for macOS 14.x where screen recording = audio access

The function tries URLs in order; the first one that opens successfully wins. On macOS 14.x, `Privacy_ListenEvent` will fail to open (pane doesn't exist), and it will fall through to `Privacy_ScreenCapture`.

### Change 6: Frontend — remove `shouldProbeSystemAudioRef` guard

**File**: `src/components/PermissionOnboarding.tsx`

The backend now returns accurate `system_audio_capture_ready` during `check_permissions()`. The frontend probe (`probe_system_audio_capture`) becomes a secondary verification rather than the primary detection mechanism.

**No functional changes needed** — the existing logic already:
1. Calls `check_permissions()` first (which now returns accurate status)
2. Only probes if status ≠ `granted` AND `shouldProbeSystemAudioRef.current` is true
3. With the backend fix, step 1 will return `granted` when permission exists → probe is skipped → no wasted 1200ms

**File**: `src/components/SettingsWindow.tsx` — same reasoning, no changes needed.

**File**: `src/components/RecordingDialog.tsx` — same reasoning. The `check_permissions()` call on mount (line 84-103) will now return accurate status.

### Change 7: Update tests

**File**: `src-tauri/src/permissions.rs`

Add test cases for the new `derive_system_audio_status` state combinations that are now reachable:

```rust
#[test]
fn derived_system_audio_status_granted_via_audio_only_permission() {
    // macOS 15+: CGPreflight=false but CATap probe=true
    assert_eq!(
        derive_system_audio_status(&PermissionState::NotGranted, &PermissionState::Granted),
        PermissionState::Granted
    );
}

#[test]
fn derived_system_audio_status_granted_via_screen_recording() {
    // macOS 14.x or 15+ with full Screen Recording
    assert_eq!(
        derive_system_audio_status(&PermissionState::Granted, &PermissionState::Granted),
        PermissionState::Granted
    );
}
```

## Source Code Structure Changes

| File | Change |
|------|--------|
| `src-tauri/src/audio/cat_tap_bridge.m` | Add `catap_probe_permission()` function |
| `src-tauri/src/audio/cat_tap.rs` | Add FFI decl + `probe_permission()` wrapper |
| `src-tauri/src/audio/mod.rs` | Re-export `probe_catap_permission` |
| `src-tauri/src/permissions.rs` | Update `check_system_audio()` to call probe; update settings URLs; add tests |

## Data Model / API / Interface Changes

**Backend `PermissionStatus` struct** — no structural changes. Fields remain the same, but `system_audio_capture_ready` will now return `Granted` or `NotGranted` instead of always `Unknown`.

**Frontend** — no interface changes. The existing `PermissionStatus` TypeScript interface already accepts all three states for `system_audio_capture_ready`.

**Tauri commands** — no new commands. `check_permissions` returns richer data; `probe_system_audio_capture` remains available for manual re-check.

## Delivery Phases

### Phase 1: Backend probe + permission detection fix
1. Add `catap_probe_permission()` to `cat_tap_bridge.m`
2. Add FFI binding + wrapper in `cat_tap.rs` + re-export in `mod.rs`
3. Update `check_system_audio()` in `permissions.rs` to use probe
4. Update settings URLs to include `Privacy_ListenEvent`
5. Add/update unit tests
6. Verify with `cargo test` and `cargo build`

### Phase 2: Verification
1. Build the app on macOS 15+ with "System Audio Recording Only" granted
2. Verify permission UI shows green "Granted"
3. Verify settings URL opens the correct pane
4. Verify no regression on macOS 14.x behavior

## Verification Approach

- `cargo test -p laconote-desktop` — runs existing + new unit tests for `derive_system_audio_status`
- `cargo build` — ensures FFI linkage is correct
- Manual testing on macOS 15+ with "System Audio Recording Only" permission
- The `catap_probe_permission()` function itself can only be meaningfully tested on macOS with actual permissions (not unit-testable in CI without TCC access)
