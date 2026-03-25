import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';
import { useEffect, useState } from 'react';
import { t } from '../i18n';
import { getStore } from '../hooks/useStore';
import { useAuth } from '../hooks/useAuth';
import { AudioTab } from './settings/AudioTab';
import { AccountTab } from './settings/AccountTab';
import { GeneralTab } from './settings/GeneralTab';
import { PermissionsTab } from './settings/PermissionsTab';
import { ShadowTab } from './settings/ShadowTab';
import { CalendarTab } from './settings/CalendarTab';

type Tab = 'audio' | 'account' | 'general' | 'permissions' | 'shadow' | 'calendar';

export function SettingsWindow() {
  const [activeTab, setActiveTab] = useState<Tab>('audio');
  const { auth } = useAuth();
  const [devices, setDevices] = useState<string[]>([]);
  const [selectedDevice, setSelectedDevice] = useState('');
  const [systemGain, setSystemGain] = useState(0.8);
  const [micGain, setMicGain] = useState(1.0);
  const [launchAtLogin, setLaunchAtLogin] = useState(false);

  useEffect(() => {
    const store = getStore();
    invoke<string[]>('list_audio_devices').then(setDevices).catch(console.error);
    isEnabled().then(setLaunchAtLogin).catch(console.error);

    store.get<string>('mic_device').then((v) => { if (v) setSelectedDevice(v); });
    store.get<number>('system_gain').then((v) => { if (v != null) setSystemGain(v); });
    store.get<number>('mic_gain').then((v) => { if (v != null) setMicGain(v); });
  }, []);

  const handleDeviceChange = async (device: string) => {
    setSelectedDevice(device);
    await getStore().set('mic_device', device);
  };

  const handleSystemGainChange = async (value: number) => {
    setSystemGain(value);
    await getStore().set('system_gain', value);
  };

  const handleMicGainChange = async (value: number) => {
    setMicGain(value);
    await getStore().set('mic_gain', value);
  };

  const handleLaunchAtLoginToggle = async () => {
    const next = !launchAtLogin;
    setLaunchAtLogin(next);
    try {
      if (next) {
        await enable();
      } else {
        await disable();
      }
    } catch {
      setLaunchAtLogin(!next);
    }
  };

  const handleLogin = () => {
    invoke('login').catch(console.error);
  };

  const handleLogout = async () => {
    await invoke('logout');
  };

  const handleClose = () => {
    getCurrentWindow().hide();
  };

  return (
    <div className="view view--gap-0">
      <div className="settings-header">
        <p className="view-title">{t.settings.title}</p>
        <button className="btn btn-secondary btn-sm" onClick={handleClose}>
          {t.settings.close}
        </button>
      </div>

      <div className="tabs">
        {(['audio', 'account', 'general', 'permissions', 'shadow', 'calendar'] as Tab[]).map((tab) => (
          <button
            key={tab}
            className={`tab${activeTab === tab ? ' active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab === 'permissions' ? t.permissions.settingsTab : tab === 'shadow' ? t.shadow.tab : t.settings[tab as 'audio' | 'account' | 'general' | 'calendar']}
          </button>
        ))}
      </div>

      <div className="tab-content">
        {activeTab === 'audio' && (
          <AudioTab
            devices={devices}
            selectedDevice={selectedDevice}
            systemGain={systemGain}
            micGain={micGain}
            onDeviceChange={handleDeviceChange}
            onSystemGainChange={handleSystemGainChange}
            onMicGainChange={handleMicGainChange}
          />
        )}
        {activeTab === 'account' && (
          <AccountTab
            auth={auth}
            onLogin={handleLogin}
            onLogout={handleLogout}
          />
        )}
        {activeTab === 'general' && (
          <GeneralTab
            launchAtLogin={launchAtLogin}
            onLaunchAtLoginToggle={handleLaunchAtLoginToggle}
          />
        )}
        {activeTab === 'permissions' && <PermissionsTab />}
        {activeTab === 'shadow' && <ShadowTab />}
        {activeTab === 'calendar' && <CalendarTab />}
      </div>
    </div>
  );
}
