import { t } from '../../i18n';

interface GeneralTabProps {
  launchAtLogin: boolean;
  onLaunchAtLoginToggle: () => void;
}

export function GeneralTab({ launchAtLogin, onLaunchAtLoginToggle }: GeneralTabProps) {
  return (
    <>
      <div className="toggle-row">
        <span className="toggle-label">{t.settings.launchAtLogin}</span>
        <input
          type="checkbox"
          className="toggle"
          checked={launchAtLogin}
          onChange={onLaunchAtLoginToggle}
        />
      </div>

      <div className="field-group">
        <p className="label">{t.settings.shortcutLabel}</p>
        <div className="shortcut-badge mt-4">
          <span className="key">⌘</span>
          <span className="key">⇧</span>
          <span className="key">R</span>
        </div>
      </div>
    </>
  );
}
