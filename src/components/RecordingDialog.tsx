import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useRef, useState } from 'react';
import { useAuth } from '../hooks/useAuth';
import { getStore } from '../hooks/useStore';
import { t, MEETING_TYPES, MeetingType } from '../i18n';
import type { AudioDiagnostic, Project } from '../types';

const API_BASE = 'https://meet.laconote.com';
const store = getStore();

interface RecordingDialogProps {
  onAuthChange?: (isAuthenticated: boolean) => void;
}

function isAuthError(err: string): boolean {
  const lower = err.toLowerCase();
  return lower.includes('not authenticated') ||
    lower.includes('session expired') ||
    lower.includes('log in') ||
    lower.includes('401') ||
    lower.includes('unauthorized');
}

function isSystemAudioError(err: string): boolean {
  const lower = err.toLowerCase();
  return lower.includes('catap') || lower.includes('system audio') || lower.includes('osstatus');
}

function isPermissionError(err: string): boolean {
  const lower = err.toLowerCase();
  return lower.includes('permission') || lower.includes('denied');
}

export function RecordingDialog({ onAuthChange }: RecordingDialogProps = {}) {
  const { auth, refresh: refreshAuth } = useAuth({ pollWhenUnauthenticated: true });
  const [meetingName, setMeetingName] = useState('');
  const [meetingType, setMeetingType] = useState<MeetingType>('general');
  const [projectId, setProjectId] = useState<string>('');
  const [projects, setProjects] = useState<Project[]>([]);
  const [loadingProjects, setLoadingProjects] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [permissionsChecked, setPermissionsChecked] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [diagnostic, setDiagnostic] = useState<AudioDiagnostic | null>(null);
  const [showDiagnostic, setShowDiagnostic] = useState(false);
  const wasUnauthRef = useRef(false);

  useEffect(() => {
    if (auth === null) return;
    const title = auth.is_authenticated ? 'Start Recording' : 'Laconote — Log In';
    getCurrentWindow().setTitle(title).catch(console.error);

    if (!auth.is_authenticated) {
      wasUnauthRef.current = true;
      return;
    }
    if (wasUnauthRef.current) {
      wasUnauthRef.current = false;
      getCurrentWindow().show().catch(console.error);
      getCurrentWindow().setFocus().catch(console.error);
      if (onAuthChange) onAuthChange(true);
    }
  }, [auth, onAuthChange]);

  useEffect(() => {
    if (!auth?.is_authenticated) return;

    invoke<{
      system_audio?: string;
      system_audio_status?: string;
      microphone: string;
    }>('check_permissions')
      .then((status) => {
        const systemAudioState = status.system_audio_status ?? status.system_audio;
        if (systemAudioState === 'granted' && status.microphone === 'granted') {
          setShowOnboarding(false);
        } else {
          setShowOnboarding(true);
        }
      })
      .catch(() => {
        setShowOnboarding(false);
      })
      .finally(() => {
        setPermissionsChecked(true);
      });
  }, [auth?.is_authenticated]);

  useEffect(() => {
    if (!auth?.is_authenticated) return;

    store.get<MeetingType>('last_meeting_type').then((v) => {
      if (v) setMeetingType(v);
    });
    store.get<string>('last_project_id').then((v) => {
      if (v) setProjectId(v);
    });

    setLoadingProjects(true);
    invoke<string | null>('get_token')
      .then((token) => {
        if (!token) {
          void refreshAuth();
          return [];
        }
        return fetch(`${API_BASE}/api/v1/projects`, {
          headers: { Authorization: `Bearer ${token}` },
        }).then((r) => {
          if (r.status === 401) {
            invoke('logout').then(() => refreshAuth()).catch(console.error);
            return [];
          }
          return r.json().then((data: { projects?: Project[] }) => data.projects ?? []);
        });
      })
      .then((list) => {
        setProjects(list);
        setLoadingProjects(false);
      })
      .catch(() => setLoadingProjects(false));
  }, [auth, refreshAuth]);

  const handleLogin = () => {
    invoke('login').catch(console.error);
  };

  const handleReLogin = async () => {
    setError(null);
    await invoke('logout').catch(console.error);
    await refreshAuth();
    invoke('login').catch(console.error);
  };

  const handleCancel = () => {
    getCurrentWindow().hide();
  };

  const handleStart = async () => {
    setError(null);
    setStarting(true);
    try {
      await store.set('last_meeting_type', meetingType);
      if (projectId) await store.set('last_project_id', projectId);

      await invoke('start_recording', {
        meetingName: meetingName.trim() || null,
        meetingType,
        projectId: projectId || null,
        micDevice: null,
      });
      await getCurrentWindow().hide();
    } catch (err) {
      const errStr = String(err);
      setError(errStr);
      setStarting(false);
      if (isAuthError(errStr)) {
        void refreshAuth();
      }
    }
  };

  if (!auth) {
    return (
      <div className="view view--centered">
        <p className="text-secondary text-center">Loading…</p>
      </div>
    );
  }

  if (!auth.is_authenticated) {
    return (
      <div className="auth-prompt">
        <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
          <circle cx="20" cy="20" r="20" fill="#0071e3" fillOpacity="0.1" />
          <path d="M20 12a4 4 0 0 1 4 4v4H16v-4a4 4 0 0 1 4-4z" fill="#0071e3" />
          <rect x="13" y="20" width="14" height="9" rx="2" fill="#0071e3" />
        </svg>
        <p className="view-title">{t.recording.notAuthenticated}</p>
        <p>{t.recording.notAuthenticatedDesc}</p>
        <button className="btn btn-primary" onClick={handleLogin}>
          {t.recording.login}
        </button>
      </div>
    );
  }

  if (!permissionsChecked) {
    return (
      <div className="view view--centered">
        <p className="text-secondary text-center">Loading…</p>
      </div>
    );
  }

  return (
    <div className="view">
      <p className="view-title">{t.recording.title}</p>

      {showOnboarding && (
        <div style={{ background: 'var(--bg-warning, #fff3cd)', padding: '8px 12px', borderRadius: '8px', marginBottom: '8px', fontSize: '12px' }}>
          <p style={{ margin: 0, fontWeight: 500 }}>{t.permissions.someNotGranted}</p>
          <button
            className="btn btn-secondary btn-xs"
            style={{ marginTop: '6px' }}
            onClick={() => {
              invoke('open_system_settings', { pane: 'system_audio' }).catch(console.error);
            }}
          >
            {t.permissions.openSettings}
          </button>
        </div>
      )}

      <div className="field-group">
        <p className="label">{t.recording.meetingName}</p>
        <input
          type="text"
          placeholder={t.recording.meetingNamePlaceholder}
          value={meetingName}
          onChange={(e) => setMeetingName(e.target.value)}
        />
      </div>

      <div className="field-group">
        <p className="label">{t.recording.meetingType}</p>
        <div className="chips">
          {MEETING_TYPES.map((type) => (
            <button
              key={type}
              className={`chip${meetingType === type ? ' active' : ''}`}
              onClick={() => setMeetingType(type)}
            >
              {t.types[type]}
            </button>
          ))}
        </div>
      </div>

      <div className="field-group">
        <p className="label">{t.recording.project}</p>
        <select value={projectId} onChange={(e) => setProjectId(e.target.value)}>
          <option value="">{loadingProjects ? t.recording.loadingProjects : t.recording.noProject}</option>
          {projects.map((p) => (
            <option key={p.project_id} value={p.project_id}>
              {p.project_name}
            </option>
          ))}
        </select>
      </div>

      {error && (
        <div className="flex-col gap-4">
          <p className="text-danger m-0" style={{ fontSize: '12px' }}>{error}</p>
          {isAuthError(error) ? (
            <button
              className="btn btn-primary btn-xs self-start"
              onClick={handleReLogin}
            >
              {t.recording.loginAgain}
            </button>
          ) : (
            <>
              {isPermissionError(error) && (
                <button
                  className="btn btn-secondary btn-xs self-start"
                  onClick={() => {
                    const pane = error.toLowerCase().includes('microphone')
                      ? 'microphone'
                      : 'system_audio';
                    invoke('open_system_settings', { pane }).catch(console.error);
                  }}
                >
                  {t.recording.openSettings}
                </button>
              )}
              {(isSystemAudioError(error) || isPermissionError(error)) && (
                <button
                  className="btn btn-secondary btn-xs self-start"
                  style={{ fontSize: '11px' }}
                  onClick={() => {
                    if (!showDiagnostic) {
                      invoke<AudioDiagnostic>('get_audio_diagnostic')
                        .then(setDiagnostic)
                        .catch(console.error);
                    }
                    setShowDiagnostic(!showDiagnostic);
                  }}
                >
                  {showDiagnostic ? 'Hide diagnostic' : 'Show diagnostic'}
                </button>
              )}
              {showDiagnostic && diagnostic && (
                <div style={{ fontSize: '11px', background: 'var(--bg-secondary, #f5f5f5)', padding: '8px', borderRadius: '6px', fontFamily: 'monospace' }}>
                  <div>macOS: {diagnostic.macos_version}</div>
                  <div>CATap available: {String(diagnostic.catap_available)}</div>
                  <div>CATap permission: {String(diagnostic.catap_permission_probe)}</div>
                  <div>Screen capture: {String(diagnostic.screen_capture_preflight)}</div>
                  <div>Mic authorized: {String(diagnostic.mic_authorized)}</div>
                  <div>Dev build: {String(diagnostic.is_dev_build)}</div>
                  {diagnostic.last_catap_error && (
                    <div style={{ color: 'var(--danger, #ff3b30)', marginTop: '4px' }}>
                      Last error: {diagnostic.last_catap_error}
                    </div>
                  )}
                  <button
                    className="btn btn-secondary btn-xs"
                    style={{ fontSize: '10px', marginTop: '4px' }}
                    onClick={() => {
                      navigator.clipboard.writeText(JSON.stringify(diagnostic, null, 2)).catch(console.error);
                    }}
                  >
                    Copy to clipboard
                  </button>
                </div>
              )}
            </>
          )}
        </div>
      )}

      <div className="btn-row">
        <button className="btn btn-secondary" onClick={handleCancel} disabled={starting}>
          {t.recording.cancel}
        </button>
        <button className="btn btn-primary" onClick={handleStart} disabled={starting}>
          {starting ? 'Starting…' : t.recording.start}
        </button>
      </div>
    </div>
  );
}
