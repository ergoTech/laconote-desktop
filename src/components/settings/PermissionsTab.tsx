import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { t } from '../../i18n';
import { usePermissions } from '../../hooks/usePermissions';
import { SettingsPermissionRow } from '../SettingsPermissionRow';

export function PermissionsTab() {
  const { status, refresh, systemAudioState, markShouldProbe } = usePermissions();
  const [grantingMic, setGrantingMic] = useState(false);

  const handleOpenScreenSettings = () => {
    markShouldProbe();
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

  const allGranted =
    systemAudioState === 'granted' && status?.microphone === 'granted';

  return (
    <div className="flex-col gap-12">
      <div>
        <p className="label mb-2">{t.permissions.settingsTitle}</p>
        <p className="note-text">
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

      <div className="flex-row gap-8">
        <div className={`status-dot ${allGranted ? 'status-dot--granted' : 'status-dot--pending'}`} />
        <p className="note-text">
          {allGranted ? t.permissions.allGranted : t.permissions.someNotGranted}
        </p>
      </div>

      <p className="revoke-note">
        {t.permissions.revokeNote}
      </p>

      <button
        className="btn btn-secondary btn-xs self-start"
        onClick={() => {
          void refresh(true);
        }}
      >
        {t.permissions.checkAgain}
      </button>
    </div>
  );
}
