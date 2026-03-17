import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';
import { LazyStore } from '@tauri-apps/plugin-store';
import { useCallback, useEffect, useRef, useState } from 'react';
import { t } from '../i18n';

const store = new LazyStore('app-settings.json');

type Tab = 'audio' | 'account' | 'general' | 'permissions' | 'shadow';

type PermissionState = 'granted' | 'not_granted' | 'unknown';

interface PermissionStatus {
  system_audio?: PermissionState;
  system_audio_status?: PermissionState;
  screen_capture_access?: PermissionState;
  system_audio_capture_ready?: PermissionState;
  microphone: PermissionState;
}

interface SystemAudioProbeResult {
  state: 'ready' | 'not_ready' | 'unknown';
  detail: string;
}

function getSystemAudioState(status: PermissionStatus | null | undefined): PermissionState {
  return status?.system_audio_status ?? status?.system_audio ?? 'unknown';
}

function mergeSystemAudioProbeResult(
  status: PermissionStatus,
  probe: SystemAudioProbeResult,
): PermissionStatus {
  if (probe.state === 'unknown') {
    return status;
  }

  if (probe.state === 'ready') {
    return {
      ...status,
      system_audio: 'granted',
      system_audio_status: 'granted',
      system_audio_capture_ready: 'granted',
    };
  }

  const currentState = getSystemAudioState(status);
  const nextState = currentState === 'granted' ? 'granted' : 'not_granted';

  return {
    ...status,
    system_audio: nextState,
    system_audio_status: nextState,
    system_audio_capture_ready: 'not_granted',
  };
}

interface AuthStatus {
  is_authenticated: boolean;
  user_email: string | null;
}

export function SettingsWindow() {
  const [activeTab, setActiveTab] = useState<Tab>('audio');
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [selectedDevice, setSelectedDevice] = useState('');
  const [systemGain, setSystemGain] = useState(0.8);
  const [micGain, setMicGain] = useState(1.0);
  const [launchAtLogin, setLaunchAtLogin] = useState(false);

  useEffect(() => {
    invoke<AuthStatus>('get_auth_status').then(setAuth).catch(console.error);
    invoke<string[]>('list_audio_devices').then(setDevices).catch(console.error);
    isEnabled().then(setLaunchAtLogin).catch(console.error);

    store.get<string>('mic_device').then((v) => { if (v) setSelectedDevice(v); });
    store.get<number>('system_gain').then((v) => { if (v != null) setSystemGain(v); });
    store.get<number>('mic_gain').then((v) => { if (v != null) setMicGain(v); });
  }, []);

  const handleDeviceChange = async (device: string) => {
    setSelectedDevice(device);
    await store.set('mic_device', device);
  };

  const handleSystemGainChange = async (value: number) => {
    setSystemGain(value);
    await store.set('system_gain', value);
  };

  const handleMicGainChange = async (value: number) => {
    setMicGain(value);
    await store.set('mic_gain', value);
  };

  const handleLaunchAtLoginToggle = async () => {
    const next = !launchAtLogin;
    setLaunchAtLogin(next);
    try {
      if (next) {
        await enable();
      } else {
        await disable();
      }
    } catch {
      setLaunchAtLogin(!next);
    }
  };

  const handleLogin = () => {
    invoke('login').catch(console.error);
  };

  const handleLogout = async () => {
    await invoke('logout');
    invoke<AuthStatus>('get_auth_status').then(setAuth).catch(console.error);
  };

  const handleClose = () => {
    getCurrentWindow().hide();
  };

  return (
    <div className="view" style={{ gap: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
        <p className="view-title">{t.settings.title}</p>
        <button className="btn btn-secondary" style={{ padding: '4px 10px', fontSize: '12px' }} onClick={handleClose}>
          {t.settings.close}
        </button>
      </div>

      <div className="tabs">
        {(['audio', 'account', 'general', 'permissions', 'shadow'] as Tab[]).map((tab) => (
          <button
            key={tab}
            className={`tab${activeTab === tab ? ' active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab === 'permissions' ? t.permissions.settingsTab : tab === 'shadow' ? t.shadow.tab : t.settings[tab as 'audio' | 'account' | 'general']}
          </button>
        ))}
      </div>

      <div className="tab-content">
        {activeTab === 'audio' && (
          <AudioTab
            devices={devices}
            selectedDevice={selectedDevice}
            systemGain={systemGain}
            micGain={micGain}
            onDeviceChange={handleDeviceChange}
            onSystemGainChange={handleSystemGainChange}
            onMicGainChange={handleMicGainChange}
          />
        )}
        {activeTab === 'account' && (
          <AccountTab
            auth={auth}
            onLogin={handleLogin}
            onLogout={handleLogout}
          />
        )}
        {activeTab === 'general' && (
          <GeneralTab
            launchAtLogin={launchAtLogin}
            onLaunchAtLoginToggle={handleLaunchAtLoginToggle}
          />
        )}
        {activeTab === 'permissions' && <PermissionsTab />}
        {activeTab === 'shadow' && <ShadowTab />}
      </div>
    </div>
  );
}

interface AudioTabProps {
  devices: string[];
  selectedDevice: string;
  systemGain: number;
  micGain: number;
  onDeviceChange: (device: string) => void;
  onSystemGainChange: (value: number) => void;
  onMicGainChange: (value: number) => void;
}

function AudioTab({ devices, selectedDevice, systemGain, micGain, onDeviceChange, onSystemGainChange, onMicGainChange }: AudioTabProps) {
  return (
    <>
      <div className="field-group">
        <p className="label">{t.settings.micDevice}</p>
        <select value={selectedDevice} onChange={(e) => onDeviceChange(e.target.value)}>
          <option value="">{t.settings.defaultMic}</option>
          {devices.map((d) => (
            <option key={d} value={d}>{d}</option>
          ))}
        </select>
      </div>

      <div className="slider-row">
        <div className="slider-row-header">
          <p className="label">{t.settings.systemGain}</p>
          <span className="slider-value">{Math.round(systemGain * 100)}%</span>
        </div>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={systemGain}
          onChange={(e) => onSystemGainChange(parseFloat(e.target.value))}
        />
      </div>

      <div className="slider-row">
        <div className="slider-row-header">
          <p className="label">{t.settings.micGain}</p>
          <span className="slider-value">{Math.round(micGain * 100)}%</span>
        </div>
        <input
          type="range"
          min={0}
          max={1.5}
          step={0.05}
          value={micGain}
          onChange={(e) => onMicGainChange(parseFloat(e.target.value))}
        />
      </div>
    </>
  );
}

interface AccountTabProps {
  auth: AuthStatus | null;
  onLogin: () => void;
  onLogout: () => void;
}

function AccountTab({ auth, onLogin, onLogout }: AccountTabProps) {
  if (!auth) {
    return <p style={{ color: 'var(--text-secondary)' }}>Loading…</p>;
  }

  if (!auth.is_authenticated) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        <p style={{ color: 'var(--text-secondary)' }}>{t.settings.notLoggedIn}</p>
        <button className="btn btn-primary" onClick={onLogin} style={{ alignSelf: 'flex-start' }}>
          {t.settings.login}
        </button>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
      <div className="account-info">
        <p className="label">{t.settings.loggedInAs}</p>
        <p className="account-email">{auth.user_email}</p>
      </div>
      <button className="btn btn-secondary" onClick={onLogout} style={{ alignSelf: 'flex-start' }}>
        {t.settings.logout}
      </button>
    </div>
  );
}

interface GeneralTabProps {
  launchAtLogin: boolean;
  onLaunchAtLoginToggle: () => void;
}

function GeneralTab({ launchAtLogin, onLaunchAtLoginToggle }: GeneralTabProps) {
  return (
    <>
      <div className="toggle-row">
        <span className="toggle-label">{t.settings.launchAtLogin}</span>
        <input
          type="checkbox"
          className="toggle"
          checked={launchAtLogin}
          onChange={onLaunchAtLoginToggle}
        />
      </div>

      <div className="field-group">
        <p className="label">{t.settings.shortcutLabel}</p>
        <div className="shortcut-badge" style={{ marginTop: '4px' }}>
          <span className="key">⌘</span>
          <span className="key">⇧</span>
          <span className="key">R</span>
        </div>
      </div>
    </>
  );
}

function PermissionsTab() {
  const [status, setStatus] = useState<PermissionStatus | null>(null);
  const [grantingMic, setGrantingMic] = useState(false);
  const shouldProbeSystemAudioRef = useRef(false);

  const refresh = useCallback(async (forceSystemAudioProbe = false) => {
    try {
      let nextStatus = await invoke<PermissionStatus>('check_permissions');
      const shouldProbeSystemAudio =
        getSystemAudioState(nextStatus) !== 'granted' &&
        (forceSystemAudioProbe || shouldProbeSystemAudioRef.current);

      if (shouldProbeSystemAudio) {
        try {
          const probe = await invoke<SystemAudioProbeResult>('probe_system_audio_capture');
          nextStatus = mergeSystemAudioProbeResult(nextStatus, probe);
        } catch (error) {
          console.error(error);
        } finally {
          shouldProbeSystemAudioRef.current = false;
        }
      }

      setStatus(nextStatus);
    } catch (error) {
      console.error(error);
    }
  }, []);

  useEffect(() => {
    void refresh();

    let unlistenFocus: (() => void) | null = null;
    getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused && shouldProbeSystemAudioRef.current) {
        void refresh(true);
      }
    }).then((fn) => { unlistenFocus = fn; }).catch(console.error);

    return () => {
      if (unlistenFocus) unlistenFocus();
    };
  }, [refresh]);

  const handleOpenScreenSettings = () => {
    shouldProbeSystemAudioRef.current = true;
    invoke('open_system_settings', { pane: 'system_audio' }).catch(console.error);
  };

  const handleOpenMicSettings = () => {
    invoke('open_system_settings', { pane: 'microphone' }).catch(console.error);
  };

  const handleGrantMic = async () => {
    setGrantingMic(true);
    try {
      await invoke('request_mic_permission');
      await new Promise((r) => setTimeout(r, 600));
      await refresh();
    } finally {
      setGrantingMic(false);
    }
  };

  const systemAudioState = getSystemAudioState(status);
  const allGranted =
    systemAudioState === 'granted' && status?.microphone === 'granted';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
      <div>
        <p className="label" style={{ marginBottom: '2px' }}>{t.permissions.settingsTitle}</p>
        <p style={{ margin: 0, fontSize: '10px', color: 'var(--text-secondary)', lineHeight: 1.4 }}>
          {t.permissions.settingsSubtitle}
        </p>
      </div>

      <SettingsPermissionRow
        icon="🔊"
        label={t.permissions.systemAudio}
        description={t.permissions.systemAudioDesc}
        state={systemAudioState}
        onGrant={handleOpenScreenSettings}
        onOpenSettings={handleOpenScreenSettings}
        grantLabel={t.permissions.openSettings}
      />

      <SettingsPermissionRow
        icon="🎙️"
        label={t.permissions.microphone}
        description={t.permissions.microphoneDesc}
        state={status?.microphone ?? 'not_granted'}
        onGrant={grantingMic ? undefined : handleGrantMic}
        onOpenSettings={handleOpenMicSettings}
        grantLabel={grantingMic ? '…' : t.permissions.grantMic}
      />

      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <div
          style={{
            width: '6px',
            height: '6px',
            borderRadius: '50%',
            background: allGranted ? '#34c759' : '#ff9f0a',
            flexShrink: 0,
          }}
        />
        <p style={{ margin: 0, fontSize: '10px', color: 'var(--text-secondary)' }}>
          {allGranted ? t.permissions.allGranted : t.permissions.someNotGranted}
        </p>
      </div>

      <p style={{ margin: '4px 0 0', fontSize: '10px', color: 'var(--text-secondary)', opacity: 0.7, lineHeight: 1.4 }}>
        {t.permissions.revokeNote}
      </p>

      <button
        className="btn btn-secondary"
        onClick={() => {
          void refresh(true);
        }}
        style={{ alignSelf: 'flex-start', fontSize: '11px' }}
      >
        {t.permissions.checkAgain}
      </button>
    </div>
  );
}

const BUFFER_DURATION_OPTIONS = [5, 10, 15, 20, 30, 45, 60];

interface ScheduleConfig {
  enabled: boolean;
  start_time: string;
  end_time: string;
  days_bitmask: number;
  buffer_minutes: number;
}

const DAY_BITS: { label: string; bit: number }[] = [
  { label: t.shadow.mon, bit: 1 },
  { label: t.shadow.tue, bit: 2 },
  { label: t.shadow.wed, bit: 4 },
  { label: t.shadow.thu, bit: 8 },
  { label: t.shadow.fri, bit: 16 },
  { label: t.shadow.sat, bit: 32 },
  { label: t.shadow.sun, bit: 64 },
];

const WEEKDAYS_MASK = 1 + 2 + 4 + 8 + 16;
const EVERY_DAY_MASK = WEEKDAYS_MASK + 32 + 64;

function ShadowTab() {
  const [bufferMinutes, setBufferMinutes] = useState(20);
  const [scheduleEnabled, setScheduleEnabled] = useState(false);
  const [startTime, setStartTime] = useState('09:00');
  const [endTime, setEndTime] = useState('18:00');
  const [daysBitmask, setDaysBitmask] = useState(WEEKDAYS_MASK);

  useEffect(() => {
    invoke<ScheduleConfig>('get_shadow_schedule').then((cfg) => {
      setBufferMinutes(cfg.buffer_minutes);
      setScheduleEnabled(cfg.enabled);
      setStartTime(cfg.start_time);
      setEndTime(cfg.end_time);
      setDaysBitmask(cfg.days_bitmask);
    }).catch(console.error);
  }, []);

  const handleBufferChange = async (minutes: number) => {
    setBufferMinutes(minutes);
    await invoke('set_shadow_buffer_duration', { minutes });
  };

  const applySchedule = async (enabled: boolean, start: string, end: string, days: number) => {
    await invoke('set_shadow_schedule', {
      enabled,
      startTime: start,
      endTime: end,
      daysBitmask: days,
    });
  };

  const handleScheduleToggle = async () => {
    const next = !scheduleEnabled;
    setScheduleEnabled(next);
    await applySchedule(next, startTime, endTime, daysBitmask);
  };

  const handleStartTimeChange = async (value: string) => {
    setStartTime(value);
    await applySchedule(scheduleEnabled, value, endTime, daysBitmask);
  };

  const handleEndTimeChange = async (value: string) => {
    setEndTime(value);
    await applySchedule(scheduleEnabled, startTime, value, daysBitmask);
  };

  const toggleDay = async (bit: number) => {
    const next = daysBitmask ^ bit;
    setDaysBitmask(next);
    await applySchedule(scheduleEnabled, startTime, endTime, next);
  };

  const setPreset = async (mask: number) => {
    setDaysBitmask(mask);
    await applySchedule(scheduleEnabled, startTime, endTime, mask);
  };

  return (
    <>
      <div className="field-group">
        <p className="label">{t.shadow.bufferDuration}</p>
        <p style={{ margin: 0, fontSize: '10px', color: 'var(--text-secondary)', lineHeight: 1.4, marginBottom: '4px' }}>
          {t.shadow.bufferDurationDesc}
        </p>
        <select value={bufferMinutes} onChange={(e) => handleBufferChange(Number(e.target.value))}>
          {BUFFER_DURATION_OPTIONS.map((m) => (
            <option key={m} value={m}>{m} {t.shadow.minutes}</option>
          ))}
        </select>
      </div>

      <div style={{ borderTop: '1px solid var(--border)', margin: '2px 0' }} />

      <div className="toggle-row">
        <span className="toggle-label">{t.shadow.scheduleEnabled}</span>
        <input
          type="checkbox"
          className="toggle"
          checked={scheduleEnabled}
          onChange={handleScheduleToggle}
        />
      </div>

      <div className="field-group" style={{ opacity: scheduleEnabled ? 1 : 0.5 }}>
        <div style={{ display: 'flex', gap: '12px' }}>
          <div style={{ flex: 1 }}>
            <p className="label">{t.shadow.scheduleStart}</p>
            <input
              type="time"
              value={startTime}
              onChange={(e) => handleStartTimeChange(e.target.value)}
              disabled={!scheduleEnabled}
              style={{
                width: '100%',
                padding: '7px 10px',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius-sm)',
                background: 'var(--surface)',
                color: 'var(--text)',
                fontSize: '13px',
                fontFamily: 'inherit',
              }}
            />
          </div>
          <div style={{ flex: 1 }}>
            <p className="label">{t.shadow.scheduleEnd}</p>
            <input
              type="time"
              value={endTime}
              onChange={(e) => handleEndTimeChange(e.target.value)}
              disabled={!scheduleEnabled}
              style={{
                width: '100%',
                padding: '7px 10px',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius-sm)',
                background: 'var(--surface)',
                color: 'var(--text)',
                fontSize: '13px',
                fontFamily: 'inherit',
              }}
            />
          </div>
        </div>
      </div>

      <div className="field-group" style={{ opacity: scheduleEnabled ? 1 : 0.5 }}>
        <p className="label">{t.shadow.activeDays}</p>
        <div style={{ display: 'flex', gap: '4px', marginBottom: '6px' }}>
          <button
            className={`chip${daysBitmask === WEEKDAYS_MASK ? ' active' : ''}`}
            onClick={() => setPreset(WEEKDAYS_MASK)}
            disabled={!scheduleEnabled}
          >
            {t.shadow.weekdays}
          </button>
          <button
            className={`chip${daysBitmask === EVERY_DAY_MASK ? ' active' : ''}`}
            onClick={() => setPreset(EVERY_DAY_MASK)}
            disabled={!scheduleEnabled}
          >
            {t.shadow.everyDay}
          </button>
        </div>
        <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
          {DAY_BITS.map(({ label, bit }) => (
            <button
              key={bit}
              className={`chip${(daysBitmask & bit) !== 0 ? ' active' : ''}`}
              onClick={() => toggleDay(bit)}
              disabled={!scheduleEnabled}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      <div style={{ borderTop: '1px solid var(--border)', margin: '2px 0' }} />

      <div className="field-group">
        <p className="label">{t.shadow.saveShortcut}</p>
        <div className="shortcut-badge" style={{ marginTop: '4px' }}>
          <span className="key">&#8984;</span>
          <span className="key">&#8679;</span>
          <span className="key">S</span>
        </div>
      </div>
    </>
  );
}

interface SettingsPermissionRowProps {
  icon: string;
  label: string;
  description: string;
  state: PermissionState;
  onGrant?: () => void;
  onOpenSettings: () => void;
  grantLabel: string;
}

function SettingsPermissionRow({
  icon,
  label,
  description,
  state,
  onGrant,
  onOpenSettings,
  grantLabel,
}: SettingsPermissionRowProps) {
  const isGranted = state === 'granted';
  const isUnknown = state === 'unknown';

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
        padding: '12px',
        borderRadius: '8px',
        background: isGranted ? 'rgba(52,199,89,0.07)' : 'rgba(255,255,255,0.03)',
        border: `1px solid ${isGranted ? 'rgba(52,199,89,0.25)' : 'rgba(255,255,255,0.08)'}`,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <span style={{ fontSize: '16px' }}>{icon}</span>
        <div style={{ flex: 1 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontWeight: 600, fontSize: '12px' }}>{label}</span>
            <span
              style={{
                fontSize: '10px',
                fontWeight: 700,
                color: isGranted ? '#34c759' : '#ff9f0a',
                textTransform: 'uppercase',
                letterSpacing: '0.5px',
              }}
            >
              {isGranted ? t.permissions.granted : isUnknown ? t.permissions.needsVerification : t.permissions.notGranted}
            </span>
          </div>
          <p style={{ margin: '1px 0 0', fontSize: '10px', color: 'var(--text-secondary)', lineHeight: 1.4 }}>
            {description}
          </p>
        </div>
      </div>

      {!isGranted && (
        <div style={{ display: 'flex', gap: '6px' }}>
          {onGrant && (
            <button
              className="btn btn-primary"
              onClick={onGrant}
              style={{ fontSize: '11px', padding: '4px 10px' }}
            >
              {grantLabel}
            </button>
          )}
          <button
            className="btn btn-secondary"
            onClick={onOpenSettings}
            style={{ fontSize: '11px', padding: '4px 10px' }}
          >
            {t.permissions.openSettings}
          </button>
        </div>
      )}
    </div>
  );
}
