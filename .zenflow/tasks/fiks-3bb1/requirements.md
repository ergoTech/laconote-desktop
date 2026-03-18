# PRD: Fix System Audio Permission Detection on macOS 15+

## Problem

On macOS 15 (Sequoia), Apple split the "Screen Recording" permission into two separate categories:

1. **Screen & System Audio Recording** — full screen + audio capture
2. **System Audio Recording Only** — audio-only capture (no screen)

Laconote only needs audio capture and appears under "System Audio Recording Only" in macOS settings. However, the app's permission check uses `CGPreflightScreenCaptureAccess()`, which only detects the **full Screen Recording** permission. This means:

- User grants "System Audio Recording Only" → toggle is ON in macOS settings
- `CGPreflightScreenCaptureAccess()` returns `false` (because it checks full Screen Recording, not the audio-only variant)
- `system_audio_capture_ready` is hardcoded to `Unknown`
- `derive_system_audio_status(NotGranted, Unknown)` → `Unknown`
- UI shows "Системне аудіо — Не налаштовано" with orange indicator, even though recording would actually work

## Root Cause

In `src-tauri/src/permissions.rs`, function `check_system_audio()`:

```rust
let screen_capture_access = if unsafe { CGPreflightScreenCaptureAccess() } {
    PermissionState::Granted
} else {
    PermissionState::NotGranted
};
let system_audio_capture_ready = PermissionState::Unknown; // always Unknown!
```

There is no Apple API to directly check "System Audio Recording Only". The only way to verify is to actually attempt a CATap (`AudioHardwareCreateProcessTap`), which is what `probe_capture_readiness()` does — but it's only called on-demand from the frontend after the user clicks "Open Settings", not during the initial permission check.

## Affected Files

- `src-tauri/src/permissions.rs` — `check_system_audio()` returns `Unknown` when `CGPreflightScreenCaptureAccess()` is false
- `src-tauri/src/audio/cat_tap.rs` — `probe_capture_readiness()` exists but is not used in the initial check
- `src/components/PermissionOnboarding.tsx` — probes only after user opens settings (via `shouldProbeSystemAudioRef`)
- `src/components/SettingsWindow.tsx` — same pattern, probes only on focus after settings opened
- `src/components/RecordingDialog.tsx` — checks permissions on mount, never probes

## Requirements

### R1: Accurate System Audio Permission Detection

The backend `check_permissions()` must return `Granted` for system audio when the user has granted "System Audio Recording Only" on macOS 15+, even if `CGPreflightScreenCaptureAccess()` returns `false`.

**Approach**: When `CGPreflightScreenCaptureAccess()` returns `false` on macOS 14.2+, attempt a quick CATap probe (`AudioHardwareCreateProcessTap`) as part of `check_system_audio()`. If the tap succeeds, the permission is effectively granted. This makes `system_audio_capture_ready` accurate instead of always `Unknown`.

### R2: Correct System Settings URL for macOS 15+

On macOS 15+, "System Audio Recording Only" has its own settings pane. The current `resolve_system_settings_urls("system_audio")` points to Screen Capture settings, which may confuse users. Consider adding the correct pane URL for macOS 15+ if one exists.

### R3: Frontend Should Probe on Initial Load

`RecordingDialog.tsx` checks permissions on mount but never calls `probe_system_audio_capture`. If the backend probe (R1) is insufficient or too slow, the frontend should also probe on initial load when the status is `Unknown`.

### R4: Minimal Performance Impact

The CATap probe in `check_system_audio()` should be fast. The current `probe_capture_readiness()` has a 1200ms timeout waiting for audio data. For the permission check, we only need to verify that `AudioHardwareCreateProcessTap` succeeds (returns `noErr`), which is nearly instant. We do NOT need to wait for audio frames.

## Scope

- **In scope**: Fix permission detection logic, ensure UI reflects actual permission state
- **Out of scope**: Changes to actual recording flow, UI redesign, support for macOS < 14.2

## Acceptance Criteria

1. On macOS 15+ with "System Audio Recording Only" granted: system audio shows as "Granted" (green) in the UI
2. On macOS 15+ with neither permission granted: system audio shows as "Not Granted" (orange)
3. On macOS 14.x with "Screen Recording" granted: behavior unchanged (shows as "Granted")
4. Permission check completes within reasonable time (< 500ms added latency)
5. Existing tests pass; new tests cover `derive_system_audio_status` with new states

## Technical Notes

- `AudioHardwareCreateProcessTap` returns `noErr` (0) when the app has the required permission (either Screen Recording or System Audio Recording Only)
- On failure it returns a non-zero `OSStatus` (often `-10867` = `kAudioHardwareBadObjectError` or similar)
- A tap-based probe creates and immediately destroys a tap — no audio data is read, no resources leak
- `CGPreflightScreenCaptureAccess()` remains useful as a fast positive check; the tap probe is only needed as a fallback when it returns `false`
