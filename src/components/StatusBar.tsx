import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { t } from '../i18n';

interface RecordingStatus {
  is_recording: boolean;
  duration_seconds: number;
  chunks_uploaded: number;
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

export function StatusBar() {
  const [status, setStatus] = useState<RecordingStatus>({
    is_recording: true,
    duration_seconds: 0,
    chunks_uploaded: 0,
  });
  const [stopping, setStopping] = useState(false);

  useEffect(() => {
    const poll = () => {
      invoke<RecordingStatus>('get_recording_status')
        .then(setStatus)
        .catch(console.error);
    };

    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, []);

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
      <div style={{ display: 'flex', alignItems: 'center' }}>
        <span className="recording-dot" />
        <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-secondary)' }}>
          {t.status.title}
        </span>
      </div>

      <div className="timer">{formatTime(status.duration_seconds)}</div>

      <div className="chunks-info">{t.status.chunks(status.chunks_uploaded)}</div>

      <button
        className="btn btn-danger"
        onClick={handleStop}
        disabled={stopping}
        style={{ minWidth: '140px' }}
      >
        {stopping ? t.status.stopping : t.status.stop}
      </button>
    </div>
  );
}
