import { LazyStore } from '@tauri-apps/plugin-store';

let instance: LazyStore | null = null;

export function getStore(): LazyStore {
  if (!instance) {
    instance = new LazyStore('app-settings.json');
  }
  return instance;
}
