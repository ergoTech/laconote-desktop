// Dashboard diagnostics — injected into the laconote.com webview before page load.
// Intercepts getUserMedia, getDisplayMedia, and console.error to log issues
// back to the Rust backend via tracing.

(function() {
  'use strict';

  function logToBackend(msg) {
    try {
      if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
        window.__TAURI_INTERNALS__.invoke('log_dashboard_event', { message: msg });
      }
    } catch(e) { /* ignore */ }
  }

  // Intercept getUserMedia
  var origGetUserMedia = navigator.mediaDevices && navigator.mediaDevices.getUserMedia
    ? navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices)
    : null;

  if (origGetUserMedia) {
    navigator.mediaDevices.getUserMedia = function(constraints) {
      logToBackend('[getUserMedia] called: ' + JSON.stringify(constraints));
      return origGetUserMedia(constraints).then(function(stream) {
        var tracks = stream.getTracks().map(function(t) { return t.kind + ':' + t.label; }).join(', ');
        logToBackend('[getUserMedia] OK: ' + tracks);
        return stream;
      }).catch(function(err) {
        logToBackend('[getUserMedia] FAILED: ' + err.name + ' — ' + err.message);
        throw err;
      });
    };
  }

  // Intercept getDisplayMedia
  var origGetDisplayMedia = navigator.mediaDevices && navigator.mediaDevices.getDisplayMedia
    ? navigator.mediaDevices.getDisplayMedia.bind(navigator.mediaDevices)
    : null;

  if (origGetDisplayMedia) {
    navigator.mediaDevices.getDisplayMedia = function(constraints) {
      logToBackend('[getDisplayMedia] called: ' + JSON.stringify(constraints));
      return origGetDisplayMedia(constraints).then(function(stream) {
        var tracks = stream.getTracks().map(function(t) { return t.kind + ':' + t.label; }).join(', ');
        logToBackend('[getDisplayMedia] OK: ' + tracks);
        return stream;
      }).catch(function(err) {
        logToBackend('[getDisplayMedia] FAILED: ' + err.name + ' — ' + err.message);
        throw err;
      });
    };
  }

  // Intercept console.error
  var origConsoleError = console.error;
  console.error = function() {
    origConsoleError.apply(console, arguments);
    try {
      var msg = Array.prototype.slice.call(arguments).map(String).join(' ');
      logToBackend('[console.error] ' + msg.substring(0, 500));
    } catch(e) { /* ignore */ }
  };

  // Catch unhandled errors
  window.addEventListener('error', function(e) {
    logToBackend('[window.error] ' + e.message + ' at ' + (e.filename || '') + ':' + (e.lineno || 0));
  });

  // Catch unhandled promise rejections
  window.addEventListener('unhandledrejection', function(e) {
    logToBackend('[unhandledrejection] ' + String(e.reason).substring(0, 500));
  });

  // Forward native audio levels to web app for visualization.
  // The Rust backend emits "audio-levels" events with { system_rms, mic_rms, mixed_rms }.
  // We expose this as a CustomEvent on window so the web app can listen:
  //   window.addEventListener('laconote-audio-levels', (e) => { ... e.detail ... })
  // Also store the latest value on window.__LACONOTE_AUDIO_LEVELS__ for polling.
  (function setupAudioLevelsForwarding() {
    if (!window.__TAURI_INTERNALS__) return;
    try {
      var cb = window.__TAURI_INTERNALS__.transformCallback(function(event) {
        var payload = event.payload || event;
        window.__LACONOTE_AUDIO_LEVELS__ = payload;
        window.dispatchEvent(new CustomEvent('laconote-audio-levels', { detail: payload }));
      });
      window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
        event: 'audio-levels',
        target: { kind: 'Any' },
        handler: cb
      });
      logToBackend('[diagnostics] Audio levels forwarding registered');
    } catch(e) {
      logToBackend('[diagnostics] Audio levels forwarding failed: ' + e.message);
    }
  })();

  // Expose desktop integration commands to the web dashboard.
  // The web app can call these to trigger native desktop features.
  window.__LACONOTE_DESKTOP__ = {
    available: true,

    connectGoogleCalendar: function() {
      logToBackend('[desktop] connectGoogleCalendar called from web dashboard');
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('connect_google_calendar')
        : Promise.reject('Not in desktop app');
    },

    disconnectCalendar: function() {
      logToBackend('[desktop] disconnectCalendar called from web dashboard');
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('disconnect_calendar')
        : Promise.reject('Not in desktop app');
    },

    getCalendarStatus: function() {
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('get_calendar_status')
        : Promise.reject('Not in desktop app');
    },

    getUpcomingEvents: function() {
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('get_upcoming_events')
        : Promise.reject('Not in desktop app');
    },

    startRecording: function(params) {
      logToBackend('[desktop] startRecording called from web dashboard');
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('start_recording', params || {})
        : Promise.reject('Not in desktop app');
    },

    stopRecording: function() {
      logToBackend('[desktop] stopRecording called from web dashboard');
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('stop_recording')
        : Promise.reject('Not in desktop app');
    },

    getRecordingStatus: function() {
      return window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__.invoke('get_recording_status')
        : Promise.reject('Not in desktop app');
    },

    openSettings: function() {
      logToBackend('[desktop] openSettings called from web dashboard');
      // Show native settings window
      if (window.__TAURI_INTERNALS__) {
        // There's no direct command for this, but we can use a workaround
        return Promise.resolve();
      }
      return Promise.reject('Not in desktop app');
    },
  };

  // Listen for recording state changes from Rust backend
  (function setupRecordingStateSync() {
    if (!window.__TAURI_INTERNALS__) return;
    try {
      // Listen for recording-state-changed events (emitted on start/stop)
      var cb = window.__TAURI_INTERNALS__.transformCallback(function(event) {
        var payload = event.payload || event;
        window.__LACONOTE_RECORDING_STATUS__ = payload;
        window.dispatchEvent(new CustomEvent('laconote-recording-state', { detail: payload }));
      });
      window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
        event: 'recording-state-changed',
        target: { kind: 'Any' },
        handler: cb
      });

      // Check initial recording state RIGHT NOW (on page load/refresh)
      // Poll quickly at first to minimize the gap where UI shows wrong state
      function syncRecordingState() {
        window.__TAURI_INTERNALS__.invoke('get_recording_status').then(function(status) {
          window.__LACONOTE_RECORDING_STATUS__ = status;
          window.dispatchEvent(new CustomEvent('laconote-recording-state', { detail: status }));
          if (status.is_recording) {
            logToBackend('[diagnostics] Page loaded with active recording: ' + status.duration_seconds + 's');
          }
        }).catch(function() {});
      }
      syncRecordingState();
      // Re-check after 500ms to catch any state that wasn't ready on initial load
      setTimeout(syncRecordingState, 500);

      logToBackend('[diagnostics] Recording state sync registered');
    } catch(e) {
      logToBackend('[diagnostics] Recording state sync failed: ' + e.message);
    }
  })();

  // Notify the web app that desktop integration is available
  window.dispatchEvent(new CustomEvent('laconote-desktop-ready', {
    detail: { version: '0.1.7', features: ['calendar', 'recording', 'detector'] }
  }));

  logToBackend('[diagnostics] Dashboard diagnostics script loaded');
  logToBackend('[diagnostics] Desktop bridge exposed as window.__LACONOTE_DESKTOP__');
})();
