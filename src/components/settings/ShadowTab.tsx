import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { t } from '../../i18n';
import type { ScheduleConfig } from '../../types';

const BUFFER_DURATION_OPTIONS = [5, 10, 15, 20, 30, 45, 60];

const DAY_BITS: { label: string; bit: number }[] = [
  { label: t.shadow.mon, bit: 1 },
  { label: t.shadow.tue, bit: 2 },
  { label: t.shadow.wed, bit: 4 },
  { label: t.shadow.thu, bit: 8 },
  { label: t.shadow.fri, bit: 16 },
  { label: t.shadow.sat, bit: 32 },
  { label: t.shadow.sun, bit: 64 },
];

const WEEKDAYS_MASK = 1 + 2 + 4 + 8 + 16;
const EVERY_DAY_MASK = WEEKDAYS_MASK + 32 + 64;

export function ShadowTab() {
  const [bufferMinutes, setBufferMinutes] = useState(20);
  const [scheduleEnabled, setScheduleEnabled] = useState(false);
  const [startTime, setStartTime] = useState('09:00');
  const [endTime, setEndTime] = useState('18:00');
  const [daysBitmask, setDaysBitmask] = useState(WEEKDAYS_MASK);

  useEffect(() => {
    invoke<ScheduleConfig>('get_shadow_schedule').then((cfg) => {
      setBufferMinutes(cfg.buffer_minutes);
      setScheduleEnabled(cfg.enabled);
      setStartTime(cfg.start_time);
      setEndTime(cfg.end_time);
      setDaysBitmask(cfg.days_bitmask);
    }).catch(console.error);
  }, []);

  const handleBufferChange = async (minutes: number) => {
    setBufferMinutes(minutes);
    await invoke('set_shadow_buffer_duration', { minutes });
  };

  const applySchedule = async (enabled: boolean, start: string, end: string, days: number) => {
    await invoke('set_shadow_schedule', {
      enabled,
      startTime: start,
      endTime: end,
      daysBitmask: days,
    });
  };

  const handleScheduleToggle = async () => {
    const next = !scheduleEnabled;
    setScheduleEnabled(next);
    await applySchedule(next, startTime, endTime, daysBitmask);
  };

  const handleStartTimeChange = async (value: string) => {
    setStartTime(value);
    await applySchedule(scheduleEnabled, value, endTime, daysBitmask);
  };

  const handleEndTimeChange = async (value: string) => {
    setEndTime(value);
    await applySchedule(scheduleEnabled, startTime, value, daysBitmask);
  };

  const toggleDay = async (bit: number) => {
    const next = daysBitmask ^ bit;
    setDaysBitmask(next);
    await applySchedule(scheduleEnabled, startTime, endTime, next);
  };

  const setPreset = async (mask: number) => {
    setDaysBitmask(mask);
    await applySchedule(scheduleEnabled, startTime, endTime, mask);
  };

  return (
    <>
      <div className="field-group">
        <p className="label">{t.shadow.bufferDuration}</p>
        <p className="note-text mb-4">
          {t.shadow.bufferDurationDesc}
        </p>
        <select value={bufferMinutes} onChange={(e) => handleBufferChange(Number(e.target.value))}>
          {BUFFER_DURATION_OPTIONS.map((m) => (
            <option key={m} value={m}>{m} {t.shadow.minutes}</option>
          ))}
        </select>
      </div>

      <hr className="divider" />

      <div className="toggle-row">
        <span className="toggle-label">{t.shadow.scheduleEnabled}</span>
        <input
          type="checkbox"
          className="toggle"
          checked={scheduleEnabled}
          onChange={handleScheduleToggle}
        />
      </div>

      <div className={`field-group${scheduleEnabled ? '' : ' opacity-faded'}`}>
        <div className="flex-row gap-12">
          <div className="flex-1">
            <p className="label">{t.shadow.scheduleStart}</p>
            <input
              type="time"
              className="time-input"
              value={startTime}
              onChange={(e) => handleStartTimeChange(e.target.value)}
              disabled={!scheduleEnabled}
            />
          </div>
          <div className="flex-1">
            <p className="label">{t.shadow.scheduleEnd}</p>
            <input
              type="time"
              className="time-input"
              value={endTime}
              onChange={(e) => handleEndTimeChange(e.target.value)}
              disabled={!scheduleEnabled}
            />
          </div>
        </div>
      </div>

      <div className={`field-group${scheduleEnabled ? '' : ' opacity-faded'}`}>
        <p className="label">{t.shadow.activeDays}</p>
        <div className="flex-row gap-4 mb-6">
          <button
            className={`chip${daysBitmask === WEEKDAYS_MASK ? ' active' : ''}`}
            onClick={() => setPreset(WEEKDAYS_MASK)}
            disabled={!scheduleEnabled}
          >
            {t.shadow.weekdays}
          </button>
          <button
            className={`chip${daysBitmask === EVERY_DAY_MASK ? ' active' : ''}`}
            onClick={() => setPreset(EVERY_DAY_MASK)}
            disabled={!scheduleEnabled}
          >
            {t.shadow.everyDay}
          </button>
        </div>
        <div className="flex-row gap-4" style={{ flexWrap: 'wrap' }}>
          {DAY_BITS.map(({ label, bit }) => (
            <button
              key={bit}
              className={`chip${(daysBitmask & bit) !== 0 ? ' active' : ''}`}
              onClick={() => toggleDay(bit)}
              disabled={!scheduleEnabled}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      <hr className="divider" />

      <div className="field-group">
        <p className="label">{t.shadow.saveShortcut}</p>
        <div className="shortcut-badge mt-4">
          <span className="key">&#8984;</span>
          <span className="key">&#8679;</span>
          <span className="key">S</span>
        </div>
      </div>
    </>
  );
}
