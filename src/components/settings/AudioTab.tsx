import { t } from '../../i18n';

interface AudioTabProps {
  devices: string[];
  selectedDevice: string;
  systemGain: number;
  micGain: number;
  onDeviceChange: (device: string) => void;
  onSystemGainChange: (value: number) => void;
  onMicGainChange: (value: number) => void;
}

export function AudioTab({ devices, selectedDevice, systemGain, micGain, onDeviceChange, onSystemGainChange, onMicGainChange }: AudioTabProps) {
  return (
    <>
      <div className="field-group">
        <p className="label">{t.settings.micDevice}</p>
        <select value={selectedDevice} onChange={(e) => onDeviceChange(e.target.value)}>
          <option value="">{t.settings.defaultMic}</option>
          {devices.map((d) => (
            <option key={d} value={d}>{d}</option>
          ))}
        </select>
      </div>

      <div className="slider-row">
        <div className="slider-row-header">
          <p className="label">{t.settings.systemGain}</p>
          <span className="slider-value">{Math.round(systemGain * 100)}%</span>
        </div>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={systemGain}
          onChange={(e) => onSystemGainChange(parseFloat(e.target.value))}
        />
      </div>

      <div className="slider-row">
        <div className="slider-row-header">
          <p className="label">{t.settings.micGain}</p>
          <span className="slider-value">{Math.round(micGain * 100)}%</span>
        </div>
        <input
          type="range"
          min={0}
          max={1.5}
          step={0.05}
          value={micGain}
          onChange={(e) => onMicGainChange(parseFloat(e.target.value))}
        />
      </div>
    </>
  );
}
