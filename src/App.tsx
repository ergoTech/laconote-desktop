import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { RecordingDialog } from './components/RecordingDialog';
import { SettingsWindow } from './components/SettingsWindow';
import { StatusBar } from './components/StatusBar';

interface RecordingStatus {
  is_recording: boolean;
  duration_seconds: number;
  chunks_uploaded: number;
}

const isSettingsWindow = window.location.hash === '#settings';

function RecordingView() {
  const [isRecording, setIsRecording] = useState(false);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const poll = () => {
      invoke<RecordingStatus>('get_recording_status')
        .then((s) => {
          setIsRecording(s.is_recording);
          if (!ready) setReady(true);
        })
        .catch(() => {
          if (!ready) setReady(true);
        });
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, [ready]);

  if (!ready) {
    return (
      <div className="view" style={{ justifyContent: 'center' }}>
        <p style={{ color: 'var(--text-secondary)', textAlign: 'center', fontSize: '12px' }}>
          Loading…
        </p>
      </div>
    );
  }

  return isRecording ? <StatusBar /> : <RecordingDialog />;
}

export default function App() {
  return isSettingsWindow ? <SettingsWindow /> : <RecordingView />;
}
