import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { usePermissions } from '../hooks/usePermissions';
import { t } from '../i18n';
import { PermissionCard } from './PermissionCard';

interface PermissionOnboardingProps {
  onDone: () => void;
}

export function PermissionOnboarding({ onDone }: PermissionOnboardingProps) {
  const { status, refresh, systemAudioState, markShouldProbe } = usePermissions({ pollIntervalMs: 3000 });
  const [grantingMic, setGrantingMic] = useState(false);
  const [openedScreenSettings, setOpenedScreenSettings] = useState(false);
  const [restarting, setRestarting] = useState(false);

  const handleOpenScreenSettings = () => {
    markShouldProbe();
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

  const importMeta = import.meta as ImportMeta & { env?: { DEV?: boolean } };
  const isDevelopment =
    Boolean(importMeta.env?.DEV) ||
    ((globalThis as { process?: { env?: { NODE_ENV?: string } } }).process?.env?.NODE_ENV === 'development');
  const allGranted =
    systemAudioState === 'granted' && status?.microphone === 'granted';

  return (
    <div className="view gap-10">
      <div>
        <p className="view-title mb-4">{t.permissions.title}</p>
        <p className="subtitle">
          {t.permissions.subtitle}
        </p>
      </div>

      <div className="flex-col gap-8">
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

      {isDevelopment && (
        <div className="notice-box notice-box--warning">
          <p className="subtitle">
            {t.permissions.devPermissionResetWarning}
          </p>
          <div className="flex-row gap-8 mt-4">
            <button
              className="btn btn-secondary btn-xs"
              onClick={() => refresh(true)}
            >
              {t.permissions.forceRecheck}
            </button>
          </div>
        </div>
      )}

      {openedScreenSettings && (
        <div className={`notice-box flex-between gap-10 ${systemAudioState === 'granted' ? 'notice-box--success' : 'notice-box--warning-strong'}`}>
          <p className="subtitle">
            {systemAudioState === 'granted' 
              ? t.permissions.restartRequiredGranted 
              : t.permissions.restartRequired}
          </p>
          <button
            className="btn btn-primary btn-xs nowrap flex-shrink-0"
            onClick={handleRestart}
            disabled={restarting}
          >
            {restarting ? '…' : t.permissions.restartApp}
          </button>
        </div>
      )}

      <div className="btn-row mt-4">
        <button
          className="btn btn-secondary btn-xs"
          onClick={onDone}
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
