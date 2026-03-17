import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LazyStore } from '@tauri-apps/plugin-store';
import { useEffect, useState } from 'react';
import { t, MEETING_TYPES, MeetingType } from '../i18n';
import { PermissionOnboarding } from './PermissionOnboarding';

const API_BASE = 'https://meet.laconote.com';
const store = new LazyStore('app-settings.json');

interface Project {
  project_id: string;
  project_name: string;
}

interface AuthStatus {
  is_authenticated: boolean;
  user_email: string | null;
}

interface RecordingDialogProps {
  onAuthChange?: (isAuthenticated: boolean) => void;
}

export function RecordingDialog({ onAuthChange }: RecordingDialogProps = {}) {
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [meetingName, setMeetingName] = useState('');
  const [meetingType, setMeetingType] = useState<MeetingType>('general');
  const [projectId, setProjectId] = useState<string>('');
  const [projects, setProjects] = useState<Project[]>([]);
  const [loadingProjects, setLoadingProjects] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [permissionsChecked, setPermissionsChecked] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);

  useEffect(() => {
    invoke<AuthStatus>('get_auth_status').then(setAuth).catch(console.error);
  }, []);

  // Update window title based on auth state
  useEffect(() => {
    if (auth === null) return; // loading
    const title = auth.is_authenticated ? 'Start Recording' : 'Laconote — Log In';
    getCurrentWindow().setTitle(title).catch(console.error);
  }, [auth]);

  // Listen for push notifications from Rust (deep link auth success)
  useEffect(() => {
    let unlisten: () => void;
    listen('auth-changed', () => {
      invoke<AuthStatus>('get_auth_status').then((a) => {
        setAuth(a);
        if (a.is_authenticated) {
          getCurrentWindow().show().catch(console.error);
          getCurrentWindow().setFocus().catch(console.error);
          if (onAuthChange) onAuthChange(true);
        }
      }).catch(console.error);
    }).then(u => { unlisten = u; });
    return () => { if (unlisten) unlisten(); };
  }, [onAuthChange]);

  useEffect(() => {
    if (auth?.is_authenticated) return;
    const id = setInterval(() => {
      invoke<AuthStatus>('get_auth_status').then((a) => {
        setAuth(a);
        if (a.is_authenticated) {
          // Show and focus the window now that the user is authenticated
          getCurrentWindow().show().catch(console.error);
          getCurrentWindow().setFocus().catch(console.error);
          if (onAuthChange) onAuthChange(true);
        }
      }).catch(console.error);
    }, 2000);
    return () => clearInterval(id);
  }, [auth?.is_authenticated, onAuthChange]);

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
        if (!token) return [];
        return fetch(`${API_BASE}/api/v1/projects`, {
          headers: { Authorization: `Bearer ${token}` },
        })
          .then((r) => r.json())
          .then((data: { projects?: Project[] }) => data.projects ?? []);
      })
      .then((list) => {
        setProjects(list);
        setLoadingProjects(false);
      })
      .catch(() => setLoadingProjects(false));
  }, [auth]);

  const handleLogin = () => {
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
      setError(String(err));
      setStarting(false);
    }
  };

  if (!auth) {
    return (
      <div className="view" style={{ justifyContent: 'center' }}>
        <p style={{ color: 'var(--text-secondary)', textAlign: 'center' }}>Loading…</p>
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
      <div className="view" style={{ justifyContent: 'center' }}>
        <p style={{ color: 'var(--text-secondary)', textAlign: 'center' }}>Loading…</p>
      </div>
    );
  }

  if (showOnboarding) {
    return <PermissionOnboarding onDone={() => setShowOnboarding(false)} />;
  }

  return (
    <div className="view">
      <p className="view-title">{t.recording.title}</p>

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
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <p style={{ color: 'var(--danger)', fontSize: '12px', margin: 0 }}>{error}</p>
          {(error.toLowerCase().includes('permission') ||
            error.toLowerCase().includes('denied')) && (
              <button
                className="btn btn-secondary"
                style={{ alignSelf: 'flex-start', fontSize: '11px', padding: '3px 8px' }}
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
