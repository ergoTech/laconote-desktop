import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useState } from 'react';
import type { AuthStatus } from '../types';

interface UseAuthOptions {
  pollWhenUnauthenticated?: boolean;
}

export function useAuth({ pollWhenUnauthenticated = false }: UseAuthOptions = {}) {
  const [auth, setAuth] = useState<AuthStatus | null>(null);

  const refresh = useCallback(() => {
    return invoke<AuthStatus>('get_auth_status').then(setAuth).catch(console.error);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let unlisten: () => void;
    listen('auth-changed', () => {
      void refresh();
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [refresh]);

  useEffect(() => {
    if (!pollWhenUnauthenticated || auth?.is_authenticated) return;
    const id = setInterval(() => {
      void refresh();
    }, 2000);
    return () => clearInterval(id);
  }, [pollWhenUnauthenticated, auth?.is_authenticated, refresh]);

  return { auth, refresh };
}
