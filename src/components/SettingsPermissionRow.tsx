import { t } from '../i18n';
import type { PermissionState } from '../types';

export interface SettingsPermissionRowProps {
  icon: string;
  label: string;
  description: string;
  state: PermissionState;
  onGrant?: () => void;
  onOpenSettings: () => void;
  grantLabel: string;
}

export function SettingsPermissionRow({
  icon,
  label,
  description,
  state,
  onGrant,
  onOpenSettings,
  grantLabel,
}: SettingsPermissionRowProps) {
  const isGranted = state === 'granted';
  const isUnknown = state === 'unknown';

  return (
    <div className={`settings-perm${isGranted ? ' settings-perm--granted' : ''}`}>
      <div className="settings-perm__header">
        <span className="settings-perm__icon">{icon}</span>
        <div className="flex-1">
          <div className="settings-perm__title-row">
            <span className="settings-perm__title">{label}</span>
            <span className={`settings-perm__status ${isGranted ? 'settings-perm__status--granted' : 'settings-perm__status--pending'}`}>
              {isGranted ? t.permissions.granted : isUnknown ? t.permissions.needsVerification : t.permissions.notGranted}
            </span>
          </div>
          <p className="settings-perm__desc">
            {description}
          </p>
        </div>
      </div>

      {!isGranted && (
        <div className="settings-perm__actions">
          {onGrant && (
            <button
              className="btn btn-primary btn-xs"
              onClick={onGrant}
            >
              {grantLabel}
            </button>
          )}
          <button
            className="btn btn-secondary btn-xs"
            onClick={onOpenSettings}
          >
            {t.permissions.openSettings}
          </button>
        </div>
      )}
    </div>
  );
}
