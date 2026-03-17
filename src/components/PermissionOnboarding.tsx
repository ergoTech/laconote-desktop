import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect, useRef, useState } from 'react';
import { t } from '../i18n';

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

interface PermissionCardProps {
  icon: string;
  label: string;
  description: string;
  benefits: string;
  why: string;
  state: PermissionState;
  onAction: () => void;
  actionLabel: string;
  actionDisabled?: boolean;
  onSecondaryAction?: () => void;
  secondaryActionLabel?: string;
}

function PermissionCard({
  icon,
  label,
  description,
  benefits,
  why,
  state,
  onAction,
  actionLabel,
  actionDisabled = false,
  onSecondaryAction,
  secondaryActionLabel,
}: PermissionCardProps) {
  const isGranted = state === 'granted';
  const isUnknown = state === 'unknown';

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '10px',
        padding: '14px',
        borderRadius: '10px',
        background: isGranted ? 'rgba(52, 199, 89, 0.07)' : 'rgba(255,255,255,0.04)',
        border: `1px solid ${isGranted ? 'rgba(52, 199, 89, 0.3)' : 'rgba(255,255,255,0.1)'}`,
        transition: 'all 0.2s ease',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        <span style={{ fontSize: '22px', lineHeight: 1 }}>{icon}</span>
        <div style={{ flex: 1 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontWeight: 600, fontSize: '13px' }}>{label}</span>
            <span
              style={{
                fontSize: '10px',
                fontWeight: 700,
                color: isGranted ? '#34c759' : '#ff9f0a',
                textTransform: 'uppercase',
                letterSpacing: '0.6px',
                background: isGranted ? 'rgba(52,199,89,0.12)' : 'rgba(255,159,10,0.12)',
                padding: '2px 7px',
                borderRadius: '20px',
              }}
            >
              {isGranted ? t.permissions.granted : isUnknown ? t.permissions.needsVerification : t.permissions.notGranted}
            </span>
          </div>
          <p style={{ margin: '2px 0 0', fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.4 }}>
            {description}
          </p>
        </div>
      </div>

      {!isGranted && (
        <>
          <div
            style={{
              background: 'rgba(0,113,227,0.08)',
              borderRadius: '6px',
              padding: '8px 10px',
              border: '1px solid rgba(0,113,227,0.15)',
            }}
          >
            <p style={{ margin: '0 0 3px', fontSize: '10px', fontWeight: 600, color: '#0071e3', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
              {t.permissions.benefitsTitle}
            </p>
            <p style={{ margin: 0, fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {benefits}
            </p>
          </div>

          <div style={{ display: 'flex', alignItems: 'flex-start', gap: '6px' }}>
            <span style={{ fontSize: '10px', color: 'var(--text-secondary)', marginTop: '1px', opacity: 0.7 }}>ⓘ</span>
            <p style={{ margin: 0, fontSize: '10px', color: 'var(--text-secondary)', lineHeight: 1.4, opacity: 0.8 }}>
              {why}
            </p>
          </div>

          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button
              className="btn btn-primary"
              onClick={onAction}
              disabled={actionDisabled}
              style={{ fontSize: '12px', padding: '6px 14px' }}
            >
              {actionLabel}
            </button>
            {onSecondaryAction && secondaryActionLabel && (
              <button
                className="btn btn-secondary"
                onClick={onSecondaryAction}
                style={{ fontSize: '12px', padding: '6px 14px' }}
              >
                {secondaryActionLabel}
              </button>
            )}
          </div>
        </>
      )}

      {isGranted && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ color: '#34c759', fontSize: '13px' }}>✓</span>
          <p style={{ margin: 0, fontSize: '11px', color: 'var(--text-secondary)', opacity: 0.8 }}>
            {benefits}
          </p>
        </div>
      )}
    </div>
  );
}

interface PermissionOnboardingProps {
  onDone: () => void;
}

export function PermissionOnboarding({ onDone }: PermissionOnboardingProps) {
  const [status, setStatus] = useState<PermissionStatus | null>(null);
  const [grantingMic, setGrantingMic] = useState(false);
  const [openedScreenSettings, setOpenedScreenSettings] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
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

  const startPolling = useCallback(() => {
    if (intervalRef.current) clearInterval(intervalRef.current);
    void refresh();
    intervalRef.current = setInterval(() => {
      void refresh();
    }, 3000);
  }, [refresh]);

  const stopPolling = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  useEffect(() => {
    startPolling();

    let unlistenFocus: (() => void) | null = null;
    getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        startPolling();
      } else {
        stopPolling();
      }
    }).then((fn) => { unlistenFocus = fn; }).catch(console.error);

    return () => {
      stopPolling();
      if (unlistenFocus) unlistenFocus();
    };
  }, [startPolling, stopPolling]);

  const handleOpenScreenSettings = () => {
    shouldProbeSystemAudioRef.current = true;
    invoke('open_system_settings', { pane: 'system_audio' }).catch(console.error);
    setOpenedScreenSettings(true);
  };

  const handleOpenMicSettings = () => {
    invoke('open_system_settings', { pane: 'microphone' }).catch(console.error);
  };

  const handleRestart = () => {
    setRestarting(true);
    invoke('restart_app').catch(console.error);
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
    <div className="view" style={{ gap: '10px' }}>
      <div>
        <p className="view-title" style={{ marginBottom: '4px' }}>{t.permissions.title}</p>
        <p style={{ margin: 0, fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
          {t.permissions.subtitle}
        </p>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
        <PermissionCard
          icon="🔊"
          label={t.permissions.systemAudio}
          description={t.permissions.systemAudioDesc}
          benefits={t.permissions.systemAudioBenefits}
          why={t.permissions.systemAudioWhy}
          state={systemAudioState}
          onAction={handleOpenScreenSettings}
          actionLabel={t.permissions.openSettings}
        />
        <PermissionCard
          icon="🎙️"
          label={t.permissions.microphone}
          description={t.permissions.microphoneDesc}
          benefits={t.permissions.microphoneBenefits}
          why={t.permissions.microphoneWhy}
          state={status?.microphone ?? 'not_granted'}
          onAction={handleGrantMic}
          actionLabel={grantingMic ? '…' : t.permissions.grantMic}
          actionDisabled={grantingMic}
          onSecondaryAction={handleOpenMicSettings}
          secondaryActionLabel={t.permissions.openSettings}
        />
      </div>

      {openedScreenSettings && (
        <div
          style={{
            background: systemAudioState === 'granted' ? 'rgba(52, 199, 89, 0.1)' : 'rgba(255, 159, 10, 0.1)',
            border: `1px solid ${systemAudioState === 'granted' ? 'rgba(52, 199, 89, 0.3)' : 'rgba(255, 159, 10, 0.3)'}`,
            borderRadius: '8px',
            padding: '10px 12px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '10px',
          }}
        >
          <p style={{ margin: 0, fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
            {systemAudioState === 'granted' 
              ? t.permissions.restartRequiredGranted 
              : t.permissions.restartRequired}
          </p>
          <button
            className="btn btn-primary"
            onClick={handleRestart}
            disabled={restarting}
            style={{ whiteSpace: 'nowrap', fontSize: '11px', padding: '5px 12px', flexShrink: 0 }}
          >
            {restarting ? '…' : t.permissions.restartApp}
          </button>
        </div>
      )}

      <div className="btn-row" style={{ marginTop: '4px' }}>
        <button
          className="btn btn-secondary"
          onClick={onDone}
          style={{ fontSize: '11px' }}
        >
          {t.permissions.donePartial}
        </button>
        <button
          className="btn btn-primary"
          onClick={onDone}
          disabled={!allGranted}
          title={!allGranted ? 'Grant all permissions above to continue' : undefined}
        >
          {t.permissions.done}
        </button>
      </div>
    </div>
  );
}
