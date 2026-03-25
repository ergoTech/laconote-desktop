import { useState } from 'react';
import { t, getLanguage, setLanguage, AVAILABLE_LANGUAGES } from '../../i18n';

interface GeneralTabProps {
  launchAtLogin: boolean;
  onLaunchAtLoginToggle: () => void;
}

export function GeneralTab({ launchAtLogin, onLaunchAtLoginToggle }: GeneralTabProps) {
  const [lang, setLang] = useState(getLanguage());

  const handleLanguageChange = (newLang: string) => {
    setLang(newLang);
    setLanguage(newLang);
    // Force re-render of entire app to apply translations
    window.location.reload();
  };

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
        <p className="label">{t.settings.language}</p>
        <select value={lang} onChange={(e) => handleLanguageChange(e.target.value)}>
          {AVAILABLE_LANGUAGES.map((l) => (
            <option key={l.code} value={l.code}>{l.label}</option>
          ))}
        </select>
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
