import { t } from '../../i18n';
import type { AuthStatus } from '../../types';

interface AccountTabProps {
  auth: AuthStatus | null;
  onLogin: () => void;
  onLogout: () => void;
}

export function AccountTab({ auth, onLogin, onLogout }: AccountTabProps) {
  if (!auth) {
    return <p className="text-secondary">Loading…</p>;
  }

  if (!auth.is_authenticated) {
    return (
      <div className="flex-col gap-12">
        <p className="text-secondary">{t.settings.notLoggedIn}</p>
        <button className="btn btn-primary self-start" onClick={onLogin}>
          {t.settings.login}
        </button>
      </div>
    );
  }

  return (
    <div className="flex-col gap-12">
      <div className="account-info">
        <p className="label">{t.settings.loggedInAs}</p>
        <p className="account-email">{auth.user_email}</p>
      </div>
      <button className="btn btn-secondary self-start" onClick={onLogout}>
        {t.settings.logout}
      </button>
    </div>
  );
}
