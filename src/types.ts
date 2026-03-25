export type PermissionState = 'granted' | 'not_granted' | 'unknown';

export interface PermissionStatus {
  system_audio?: PermissionState;
  system_audio_status?: PermissionState;
  screen_capture_access?: PermissionState;
  system_audio_capture_ready?: PermissionState;
  microphone: PermissionState;
}

export interface SystemAudioProbeResult {
  state: 'ready' | 'authorized_but_silent' | 'not_ready' | 'unknown';
  detail: string;
}

export interface AuthStatus {
  is_authenticated: boolean;
  user_email: string | null;
}

export interface RecordingStatus {
  is_recording: boolean;
  duration_seconds: number;
  chunks_uploaded: number;
}

export interface ScheduleConfig {
  enabled: boolean;
  start_time: string;
  end_time: string;
  days_bitmask: number;
  buffer_minutes: number;
}

export interface Project {
  project_id: string;
  project_name: string;
}

export interface AudioDiagnostic {
  macos_version: string;
  catap_available: boolean;
  catap_permission_probe: boolean;
  screen_capture_preflight: boolean;
  mic_authorized: boolean;
  is_dev_build: boolean;
  last_catap_error: string | null;
}
