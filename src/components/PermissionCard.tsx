import { t } from '../i18n';
import type { PermissionState } from '../types';

export interface PermissionCardProps {
  icon: string;
  label: string;
  description: string;
  benefits: string;
  why: string;
  state: PermissionState;
  onAction: () => void;
  actionLabel: string;
  actionDisabled?: boolean;
  onSecondaryAction?: () => void;
  secondaryActionLabel?: string;
}

export function PermissionCard({
  icon,
  label,
  description,
  benefits,
  why,
  state,
  onAction,
  actionLabel,
  actionDisabled = false,
  onSecondaryAction,
  secondaryActionLabel,
}: PermissionCardProps) {
  const isGranted = state === 'granted';
  const isUnknown = state === 'unknown';

  return (
    <div className={`permission-card${isGranted ? ' permission-card--granted' : ''}`}>
      <div className="permission-card__header">
        <span className="permission-card__icon">{icon}</span>
        <div className="flex-1">
          <div className="permission-card__title-row">
            <span className="permission-card__title">{label}</span>
            <span className={`permission-card__badge ${isGranted ? 'permission-card__badge--granted' : 'permission-card__badge--pending'}`}>
              {isGranted ? t.permissions.granted : isUnknown ? t.permissions.needsVerification : t.permissions.notGranted}
            </span>
          </div>
          <p className="permission-card__desc">
            {description}
          </p>
        </div>
      </div>

      {!isGranted && (
        <>
          <div className="permission-card__info-box">
            <p className="permission-card__info-label">
              {t.permissions.benefitsTitle}
            </p>
            <p className="permission-card__info-text">
              {benefits}
            </p>
          </div>

          <div className="permission-card__hint-row">
            <span className="permission-card__hint-icon">ⓘ</span>
            <p className="permission-card__hint-text">
              {why}
            </p>
          </div>

          <div className="permission-card__actions">
            <button
              className="btn btn-primary btn-sm"
              onClick={onAction}
              disabled={actionDisabled}
            >
              {actionLabel}
            </button>
            {onSecondaryAction && secondaryActionLabel && (
              <button
                className="btn btn-secondary btn-sm"
                onClick={onSecondaryAction}
              >
                {secondaryActionLabel}
              </button>
            )}
          </div>
        </>
      )}

      {isGranted && (
        <div className="permission-card__granted-row">
          <span className="permission-card__granted-check">✓</span>
          <p className="permission-card__granted-text">
            {benefits}
          </p>
        </div>
      )}
    </div>
  );
}
