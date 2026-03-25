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

  logToBackend('[diagnostics] Dashboard diagnostics script loaded');
})();
