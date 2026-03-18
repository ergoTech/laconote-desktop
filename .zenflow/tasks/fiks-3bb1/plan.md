# Full SDD workflow

## Configuration
- **Artifacts Path**: {@artifacts_path} → `.zenflow/tasks/{task_id}`

---

## Agent Instructions

---

## Workflow Steps

### [x] Step: Requirements
<!-- chat-id: a7044a72-52a9-4be0-a824-bc547b17d196 -->

Create a Product Requirements Document (PRD) based on the feature description.

1. Review existing codebase to understand current architecture and patterns
2. Analyze the feature definition and identify unclear aspects
3. Ask the user for clarifications on aspects that significantly impact scope or user experience
4. Make reasonable decisions for minor details based on context and conventions
5. If user can't clarify, make a decision, state the assumption, and continue

Save the PRD to `{@artifacts_path}/requirements.md`.

### [x] Step: Technical Specification
<!-- chat-id: 5f9d376a-f78b-4365-895b-3d66ee2af7ee -->

Create a technical specification based on the PRD in `{@artifacts_path}/requirements.md`.

1. Review existing codebase architecture and identify reusable components
2. Define the implementation approach

Save to `{@artifacts_path}/spec.md` with:
- Technical context (language, dependencies)
- Implementation approach referencing existing code patterns
- Source code structure changes
- Data model / API / interface changes
- Delivery phases (incremental, testable milestones)
- Verification approach using project lint/test commands

### [x] Step: Planning
<!-- chat-id: d5738795-0b80-4877-9e80-704052104d10 -->

### [x] Step: Add lightweight CATap permission probe to Objective-C bridge
<!-- chat-id: 25f52ff4-d55d-4126-97b9-8629d453c123 -->

**File**: `src-tauri/src/audio/cat_tap_bridge.m`

Add a new C function `catap_probe_permission()` that:
1. Checks `isAtLeastMacOS14_2()`, returns `0` if not
2. On `gTapQueue` (serial, same as `catap_start`/`catap_stop`): creates a `CATapDescription`, calls `AudioHardwareCreateProcessTap`
3. If `noErr` → immediately calls `AudioHardwareDestroyProcessTap` → returns `1`
4. If error → returns `0`

No IOProc, no audio device start, no callbacks. Only tests whether the tap can be created (requires Screen Recording or System Audio Recording Only permission). Expected latency < 50ms.

Must check `gTapID != kAudioObjectUnknown` first — if a tap is already active, the probe should return `1` without creating a second tap (system allows only one global tap per process).

### [x] Step: Expose probe via Rust FFI and update permission check logic
<!-- chat-id: ce24be4e-824f-41c3-8c69-eee0a1c9670c -->

**Files**: `src-tauri/src/audio/cat_tap.rs`, `src-tauri/src/audio/mod.rs`, `src-tauri/src/permissions.rs`

1. In `cat_tap.rs`: add `extern "C" { fn catap_probe_permission() -> i32; }` and `pub fn probe_permission() -> bool`
2. In `mod.rs`: re-export as `pub use cat_tap::probe_permission as probe_catap_permission;`
3. In `permissions.rs` — update `check_system_audio()`:
   - When `CGPreflightScreenCaptureAccess()` returns `true`: set both `screen_capture_access = Granted` and `system_audio_capture_ready = Granted` (full Screen Recording implies audio)
   - When `CGPreflightScreenCaptureAccess()` returns `false`: call `probe_catap_permission()` as fallback
     - If `true` → `system_audio_capture_ready = Granted`
     - If `false` → `system_audio_capture_ready = NotGranted`
4. Update settings URLs in `resolve_system_settings_urls("system_audio")`:
   - Add `Privacy_ListenEvent` (System Audio Recording Only, macOS 15+) as first entry
   - Add `Privacy_AudioCapture` as second entry
   - Keep existing URLs as fallbacks
   - Keep `screen_recording` branch unchanged (it should still point to `Privacy_ScreenCapture` first)
5. Add unit tests for `derive_system_audio_status` covering newly reachable state combinations:
   - `(NotGranted, Granted)` → `Granted` (audio-only permission on macOS 15+)
   - `(Granted, Granted)` → `Granted` (full screen recording)
6. Verify: `cargo build`, `cargo test -p laconote-desktop`

### [x] Step: Fix recording start failure after permission probe

**File**: `src-tauri/src/recording.rs`

Triple CATap create/destroy cycle (permission probe → readiness probe → actual start) caused CoreAudio resource contention on macOS 15+.

1. Skip `probe_catap_capture_readiness()` when `system_audio_capture_ready == Granted` (already confirmed by lightweight probe)
2. Add 200ms delay after readiness probe (when it does run) for CoreAudio cleanup
3. Add retry with 300ms backoff for `start_catap_capture()` in case of transient tap creation failure
4. Verify: `cargo build`, `cargo test -p laconote-desktop` — 103 tests pass
