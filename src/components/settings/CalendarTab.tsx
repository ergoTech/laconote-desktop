import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { DetectorTab } from './DetectorTab';

interface CalendarStatus {
  connected: boolean;
  provider: string;
  config: {
    enabled: boolean;
    remind_minutes_before: number;
    auto_record: boolean;
  };
}

interface CalendarEvent {
  id: string;
  summary: string;
  start_time: string;
  end_time: string;
  meeting_link: string | null;
  platform: string | null;
}

export function CalendarTab() {
  const [status, setStatus] = useState<CalendarStatus | null>(null);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    invoke<CalendarStatus>('get_calendar_status').then(setStatus).catch(console.error);
  };

  useEffect(() => {
    refresh();
    const unlisten = listen<CalendarEvent[]>('calendar-events', (e) => {
      setEvents(e.payload);
    });
    const unlistenConnected = listen('calendar-connected', () => {
      refresh();
    });
    return () => {
      unlisten.then((f) => f());
      unlistenConnected.then((f) => f());
    };
  }, []);

  const handleConnect = async () => {
    setConnecting(true);
    setError(null);
    try {
      await invoke('connect_google_calendar');
      refresh();
    } catch (err) {
      setError(String(err));
      console.error(err);
    }
    setConnecting(false);
  };

  const handleDisconnect = async () => {
    await invoke('disconnect_calendar');
    setEvents([]);
    refresh();
  };

  const handleConfigChange = (key: string, value: boolean | number) => {
    const params: Record<string, boolean | number | null> = {
      enabled: null,
      remindMinutes: null,
      autoRecord: null,
    };
    if (key === 'enabled') params.enabled = value as boolean;
    if (key === 'remindMinutes') params.remindMinutes = value as number;
    if (key === 'autoRecord') params.autoRecord = value as boolean;
    invoke('set_calendar_config', params).then(refresh).catch(console.error);
  };

  const handleFetchEvents = () => {
    invoke<CalendarEvent[]>('get_upcoming_events')
      .then(setEvents)
      .catch(console.error);
  };

  if (!status) {
    return <div className="settings-section"><p className="text-secondary">Loading...</p></div>;
  }

  return (
    <div className="settings-section">
      <p className="settings-section-title">Calendar Integration</p>
      <p className="text-secondary" style={{ fontSize: '12px', marginBottom: '12px' }}>
        Connect your calendar to get reminders before meetings and auto-start recording.
      </p>

      {!status.connected ? (
        <div>
          <button
            className="btn btn-primary"
            onClick={handleConnect}
            disabled={connecting}
          >
            {connecting ? 'Waiting for authorization...' : 'Connect Google Calendar'}
          </button>
          {connecting && (
            <p className="text-secondary" style={{ fontSize: '11px', marginTop: '8px' }}>
              Complete authorization in your browser, then return here.
            </p>
          )}
          {error && (
            <p className="text-danger" style={{ fontSize: '11px', marginTop: '8px' }}>{error}</p>
          )}
        </div>
      ) : (
        <>
          <div className="settings-row">
            <span>Google Calendar</span>
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
              <span className="text-secondary" style={{ fontSize: '12px' }}>Connected</span>
              <button className="btn btn-secondary btn-xs" onClick={handleDisconnect}>
                Disconnect
              </button>
            </div>
          </div>

          <div className="settings-row">
            <span>Calendar sync</span>
            <label className="toggle">
              <input
                type="checkbox"
                checked={status.config.enabled}
                onChange={(e) => handleConfigChange('enabled', e.target.checked)}
              />
              <span className="toggle-slider" />
            </label>
          </div>

          <div className="settings-row">
            <span>Remind before meeting</span>
            <select
              value={status.config.remind_minutes_before}
              onChange={(e) => handleConfigChange('remindMinutes', Number(e.target.value))}
            >
              <option value={1}>1 minute</option>
              <option value={2}>2 minutes</option>
              <option value={5}>5 minutes</option>
              <option value={10}>10 minutes</option>
            </select>
          </div>

          <div className="settings-row">
            <span>Auto-record meetings</span>
            <label className="toggle">
              <input
                type="checkbox"
                checked={status.config.auto_record}
                onChange={(e) => handleConfigChange('autoRecord', e.target.checked)}
              />
              <span className="toggle-slider" />
            </label>
          </div>

          {events.length > 0 && (
            <div style={{ marginTop: '12px' }}>
              <p className="settings-section-title" style={{ fontSize: '12px' }}>Upcoming meetings</p>
              {events.map((ev) => {
                const start = new Date(ev.start_time);
                const timeStr = start.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
                return (
                  <div key={ev.id} className="settings-row" style={{ fontSize: '12px' }}>
                    <span>{ev.summary}</span>
                    <span className="text-secondary">
                      {timeStr}
                      {ev.platform && ` · ${ev.platform}`}
                    </span>
                  </div>
                );
              })}
            </div>
          )}

          <button
            className="btn btn-secondary btn-xs"
            onClick={handleFetchEvents}
            style={{ marginTop: '8px' }}
          >
            Refresh events
          </button>
        </>
      )}

      <div style={{ marginTop: '20px', borderTop: '1px solid var(--border, #e0e0e0)', paddingTop: '16px' }}>
        <DetectorTab />
      </div>
    </div>
  );
}
