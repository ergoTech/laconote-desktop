import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

interface DetectorConfig {
  enabled: boolean;
  notify: boolean;
  auto_record: boolean;
}

interface MeetingDetected {
  app_name: string;
  platform: string;
}

export function DetectorTab() {
  const [config, setConfig] = useState<DetectorConfig>({ enabled: true, notify: true, auto_record: false });
  const [lastDetected, setLastDetected] = useState<MeetingDetected | null>(null);

  useEffect(() => {
    invoke<DetectorConfig>('get_detector_config').then(setConfig).catch(console.error);
    const unlisten = listen<MeetingDetected>('meeting-detected', (e) => {
      setLastDetected(e.payload);
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleChange = (key: string, value: boolean) => {
    const params: Record<string, boolean | null> = { enabled: null, notify: null, autoRecord: null };
    if (key === 'enabled') params.enabled = value;
    if (key === 'notify') params.notify = value;
    if (key === 'autoRecord') params.autoRecord = value;
    invoke('set_detector_config', params).then(() => {
      invoke<DetectorConfig>('get_detector_config').then(setConfig);
    }).catch(console.error);
  };

  return (
    <div className="settings-section">
      <p className="settings-section-title">Meeting Detection</p>
      <p className="text-secondary" style={{ fontSize: '12px', marginBottom: '12px' }}>
        Automatically detect when you join a meeting in Zoom, Google Meet, Teams, Discord, or other apps.
      </p>

      <div className="settings-row">
        <span>Auto-detect meetings</span>
        <label className="toggle">
          <input type="checkbox" checked={config.enabled} onChange={(e) => handleChange('enabled', e.target.checked)} />
          <span className="toggle-slider" />
        </label>
      </div>

      <div className="settings-row">
        <span>Show notification</span>
        <label className="toggle">
          <input type="checkbox" checked={config.notify} onChange={(e) => handleChange('notify', e.target.checked)} />
          <span className="toggle-slider" />
        </label>
      </div>

      <div className="settings-row">
        <span>Auto-record when detected</span>
        <label className="toggle">
          <input type="checkbox" checked={config.auto_record} onChange={(e) => handleChange('autoRecord', e.target.checked)} />
          <span className="toggle-slider" />
        </label>
      </div>

      {lastDetected && (
        <div style={{ marginTop: '12px', fontSize: '12px' }}>
          <p className="text-secondary">Last detected: {lastDetected.platform} ({lastDetected.app_name})</p>
        </div>
      )}

      <p className="text-secondary" style={{ fontSize: '11px', marginTop: '12px' }}>
        Supported: Zoom, Google Meet, Microsoft Teams, Discord, Slack, Webex, FaceTime
      </p>
    </div>
  );
}
