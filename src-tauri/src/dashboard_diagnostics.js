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

  logToBackend('[diagnostics] Dashboard diagnostics script loaded');
})();
