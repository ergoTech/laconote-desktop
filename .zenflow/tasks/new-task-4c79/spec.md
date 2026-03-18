# Technical Specification: Audio Permissions Fix

## 1. Technical Context
- **Stack**: Tauri (Rust backend), React (TypeScript frontend).
- **Core issue**: The macOS TCC permission flow is unstable between ad-hoc builds during development. Additionally, the app's readiness probe (`probe_system_audio_capture`) returns `Unknown` if no sound is currently playing, which the UI ignores, leaving the user stuck in the "Not Granted" state despite OS-level permission being granted.

## 2. Implementation Approach
To resolve the UX issue without migrating to native code, we need to decouple "OS permission granted" from "audio actively streaming".
1. **Refine the Probe Results**: `probe_capture_readiness` in `cat_tap.rs` should explicitly indicate when `catap_start` succeeds. If it succeeds but no frames arrive (or empty frames arrive), the OS permission is effectively **granted**, but the stream is **silent**.
2. **Update Data Models**: Change `CaptureReadiness` to distinguish between `Ready` (audio flowing), `AuthorizedButSilent` (TCC granted, but no audio), and `NotReady` (TCC denied or error). 
3. **Update UI**: `PermissionOnboarding.tsx` must treat `AuthorizedButSilent` as a granted TCC permission, allowing the user to proceed. It should optionally display a tooltip/info that no active audio was detected. We will also add a disclaimer for development builds about TCC instability.
4. **Recording Flow Alignment**: Ensure `recording.rs` treats `AuthorizedButSilent` the same as `Unknown` previously — allowing recording to start.

## 3. Data Model / API / Interface Changes

### Rust Backend
1. **`src-tauri/src/audio/cat_tap.rs`**:
   Update `CaptureReadiness` enum:
   ```rust
   pub enum CaptureReadiness {
       Ready,
       AuthorizedButSilent, // Replaces Unknown for the successful start but no audio case
       NotReady,
       Unknown, // For unexpected errors where state is truly unknown
   }
   ```

2. **`src-tauri/src/permissions.rs`**:
   Update `PermissionState` if necessary, but we can likely map `AuthorizedButSilent` to `PermissionState::Granted` for the `system_audio_status` so the UI knows TCC is fine.

### Frontend
1. **`src/components/PermissionOnboarding.tsx`**:
   - Update `SystemAudioProbeResult` state type to include `'authorized_but_silent'`.
   - Update `mergeSystemAudioProbeResult` to treat `'authorized_but_silent'` as `system_audio: 'granted'`, so the onboarding allows the user to proceed.
   - Add a UI warning/disclaimer if `process.env.NODE_ENV === 'development'` about macOS permission resets between builds.

## 4. Source Code Structure Changes
- **`src-tauri/src/audio/cat_tap.rs`**:
  Modify `probe_capture_readiness` to return `CaptureReadiness::AuthorizedButSilent` instead of `Unknown` when `catap_start` succeeds but times out waiting for frames.
- **`src-tauri/src/recording.rs`**:
  Update pattern matching on `system_audio_probe.state` to accept `AuthorizedButSilent`.
- **`src/components/PermissionOnboarding.tsx`**:
  Adjust the state merger logic and add a small dev-only disclaimer text.

## 5. Delivery Phases
- **Phase 1: Backend Data Model Update**: Modify `CaptureReadiness` in `cat_tap.rs` and update the probe logic. Fix compilation errors in `recording.rs` by handling the new enum variants.
- **Phase 2: Frontend Integration**: Update `PermissionOnboarding.tsx` to handle the new probe states. Implement the UI logic to mark the step as granted when `AuthorizedButSilent` is received.
- **Phase 3: Dev Disclaimer**: Add the dev mode warning about TCC resets in the UI.

## 6. Verification Approach
- **Unit Tests**: Run `cargo test` in `src-tauri` to ensure the `permissions.rs` tests (e.g., `derived_system_audio_status_prefers_verified_granted`) continue to pass.
- **Linting**: Run `npm run lint` and `npm run typecheck` to verify frontend TypeScript changes.
- **Manual Verification**: 
  1. Start the app.
  2. Ensure system audio is NOT playing.
  3. Open onboarding, grant permission in macOS settings.
  4. Verify the UI updates to "Granted" (instead of staying stuck) and allows completing the onboarding.