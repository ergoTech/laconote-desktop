import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { useRecordingStatus } from '../hooks/useRecordingStatus';
import { t } from '../i18n';

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

export function StatusBar() {
  const { status: polledStatus } = useRecordingStatus();
  const status = polledStatus ?? { is_recording: true, duration_seconds: 0, chunks_uploaded: 0 };
  const [stopping, setStopping] = useState(false);

  const handleStop = async () => {
    setStopping(true);
    try {
      await invoke<string>('stop_recording');
    } catch (err) {
      console.error(err);
      setStopping(false);
    }
  };

  return (
    <div className="status-bar">
      <div className="flex-row">
        <span className="recording-dot" />
        <span className="status-label">
          {t.status.title}
        </span>
      </div>

      <div className="timer">{formatTime(status.duration_seconds)}</div>

      <div className="chunks-info">{t.status.chunks(status.chunks_uploaded)}</div>

      <button
        className="btn btn-danger btn-wide"
        onClick={handleStop}
        disabled={stopping}
      >
        {stopping ? t.status.stopping : t.status.stop}
      </button>
    </div>
  );
}
