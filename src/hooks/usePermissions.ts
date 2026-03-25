import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { PermissionState, PermissionStatus, SystemAudioProbeResult } from '../types';

export function getSystemAudioState(status: PermissionStatus | null | undefined): PermissionState {
  return status?.system_audio_status ?? status?.system_audio ?? 'unknown';
}

export function mergeSystemAudioProbeResult(
  status: PermissionStatus,
  probe: SystemAudioProbeResult,
): PermissionStatus {
  if (probe.state === 'unknown') return status;

  if (probe.state === 'ready' || probe.state === 'authorized_but_silent') {
    return {
      ...status,
      system_audio: 'granted',
      system_audio_status: 'granted',
      system_audio_capture_ready: probe.state === 'ready' ? 'granted' : 'not_granted',
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

interface UsePermissionsOptions {
  pollIntervalMs?: number;
}

export function usePermissions({ pollIntervalMs }: UsePermissionsOptions = {}) {
  const [status, setStatus] = useState<PermissionStatus | null>(null);
  const shouldProbeSystemAudioRef = useRef(false);

  const refresh = useCallback(async (forceSystemAudioProbe = false) => {
    try {
      let nextStatus = await invoke<PermissionStatus>('check_permissions');
      const shouldProbe =
        getSystemAudioState(nextStatus) !== 'granted' &&
        (forceSystemAudioProbe || shouldProbeSystemAudioRef.current);

      if (shouldProbe) {
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

  const markShouldProbe = useCallback(() => {
    shouldProbeSystemAudioRef.current = true;
  }, []);

  useEffect(() => {
    void refresh();

    let intervalId: ReturnType<typeof setInterval> | null = null;

    const startPolling = () => {
      if (!pollIntervalMs) return;
      if (intervalId) clearInterval(intervalId);
      intervalId = setInterval(() => {
        void refresh();
      }, pollIntervalMs);
    };

    const stopPolling = () => {
      if (intervalId) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };

    if (pollIntervalMs) startPolling();

    let unlistenFocus: (() => void) | null = null;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) {
          if (pollIntervalMs) {
            void refresh();
            startPolling();
          } else if (shouldProbeSystemAudioRef.current) {
            void refresh(true);
          }
        } else if (pollIntervalMs) {
          stopPolling();
        }
      })
      .then((fn) => {
        unlistenFocus = fn;
      })
      .catch(console.error);

    return () => {
      stopPolling();
      if (unlistenFocus) unlistenFocus();
    };
  }, [refresh, pollIntervalMs]);

  const systemAudioState = getSystemAudioState(status);

  return { status, refresh, systemAudioState, markShouldProbe };
}
