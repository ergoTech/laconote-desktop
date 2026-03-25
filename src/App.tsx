import { RecordingDialog } from './components/RecordingDialog';
import { SettingsWindow } from './components/SettingsWindow';
import { StatusBar } from './components/StatusBar';
import { useRecordingStatus } from './hooks/useRecordingStatus';

const isSettingsWindow = window.location.hash === '#settings';

function RecordingView() {
  const { status, ready } = useRecordingStatus();

  if (!ready) {
    return (
      <div className="view view--centered">
        <p className="text-secondary text-center" style={{ fontSize: '12px' }}>
          Loading…
        </p>
      </div>
    );
  }

  return status?.is_recording ? <StatusBar /> : <RecordingDialog />;
}

export default function App() {
  return isSettingsWindow ? <SettingsWindow /> : <RecordingView />;
}
