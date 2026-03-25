import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import type { RecordingStatus } from '../types';

export function useRecordingStatus() {
  const [status, setStatus] = useState<RecordingStatus | null>(null);
  const [ready, setReady] = useState(false);
  const readyRef = useRef(false);

  useEffect(() => {
    const markReady = () => {
      if (!readyRef.current) {
        readyRef.current = true;
        setReady(true);
      }
    };

    const poll = () => {
      invoke<RecordingStatus>('get_recording_status')
        .then((s) => {
          setStatus(s);
          markReady();
        })
        .catch(() => {
          markReady();
        });
    };

    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, []);

  return { status, ready };
}
