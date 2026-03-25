#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>
#import <CoreAudio/CATapDescription.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/AudioHardware.h>
#import <AppKit/AppKit.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>

typedef void (*LacoCATapCallback)(const float* samples, int count, int channels, void* user_data);

static CATapDescription *gTapDesc = nil;
static AudioObjectID gTapID = kAudioObjectUnknown;
static AudioObjectID gAggregateDeviceID = kAudioObjectUnknown;
static AudioDeviceIOProcID gIOProcID = NULL;
static LacoCATapCallback gCallback = NULL;
static void* gUserData = NULL;
static uint32_t gSampleRate = 48000;

static dispatch_queue_t gTapQueue = NULL;
static BOOL gVersionLogged = NO;
static NSString *gLastDiagnostic = nil;

/// Decodes an OSStatus into a human-readable string.
static NSString* describeOSStatus(OSStatus status) {
    switch (status) {
        case noErr:           return @"noErr (0)";
        case -50:             return @"paramErr (-50): Invalid parameter";
        case -10867:          return @"kAudioHardwareBadObjectError (-10867): Invalid AudioObjectID";
        case -10878:          return @"kAudioHardwareNotRunningError (-10878): Hardware not running";
        case -10863:          return @"kAudioHardwareUnspecifiedError (-10863): Unspecified error";
        case -10851:          return @"kAudioHardwareUnsupportedOperationError (-10851): Unsupported operation";
        case -10861:          return @"kAudioDevicePermissionsError (-10861): Permission denied";
        case -10862:          return @"kAudioHardwareIllegalOperationError (-10862): Illegal operation";
        case -10866:          return @"kAudioHardwareBadDeviceError (-10866): Bad device";
        default: {
            char fourCC[5] = {0};
            fourCC[0] = (char)((status >> 24) & 0xFF);
            fourCC[1] = (char)((status >> 16) & 0xFF);
            fourCC[2] = (char)((status >> 8) & 0xFF);
            fourCC[3] = (char)(status & 0xFF);
            BOOL printable = YES;
            for (int i = 0; i < 4; i++) {
                if (fourCC[i] < 32 || fourCC[i] > 126) { printable = NO; break; }
            }
            if (printable) {
                return [NSString stringWithFormat:@"'%s' (%d)", fourCC, (int)status];
            }
            return [NSString stringWithFormat:@"unknown (%d)", (int)status];
        }
    }
}

/// Returns the last diagnostic message from catap_start (C string, valid until next call).
const char* catap_last_diagnostic(void) {
    if (gLastDiagnostic == nil) return "";
    return [gLastDiagnostic UTF8String];
}

__attribute__((constructor))
static void initTapQueue(void) {
    gTapQueue = dispatch_queue_create("com.laconote.catap", DISPATCH_QUEUE_SERIAL);
}

static BOOL isAtLeastMacOS14_2(void) {
    NSOperatingSystemVersion v = [[NSProcessInfo processInfo] operatingSystemVersion];
    return (v.majorVersion > 14) || (v.majorVersion == 14 && v.minorVersion >= 2);
}

static void logMacOSVersion(void) {
    if (gVersionLogged) return;
    gVersionLogged = YES;
    NSOperatingSystemVersion v = [[NSProcessInfo processInfo] operatingSystemVersion];
    NSLog(@"[Laconote CATap] macOS %ld.%ld.%ld", (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion);
}

static OSStatus ioProc(
    AudioObjectID inDevice,
    const AudioTimeStamp* inNow,
    const AudioBufferList* inInputData,
    const AudioTimeStamp* inInputTime,
    AudioBufferList* outOutputData,
    const AudioTimeStamp* inOutputTime,
    void* inClientData)
{
    (void)inDevice; (void)inNow; (void)inInputTime;
    (void)outOutputData; (void)inOutputTime; (void)inClientData;

    if (gCallback == NULL || inInputData == NULL) return noErr;

    for (UInt32 i = 0; i < inInputData->mNumberBuffers; i++) {
        const AudioBuffer* buf = &inInputData->mBuffers[i];
        if (buf->mData == NULL || buf->mDataByteSize == 0) continue;
        int count = (int)(buf->mDataByteSize / sizeof(float));
        int channels = (int)buf->mNumberChannels;
        gCallback((const float*)buf->mData, count, channels, gUserData);
    }
    return noErr;
}

static void catap_cleanup_locked(void) {
    if (gIOProcID != NULL && gAggregateDeviceID != kAudioObjectUnknown) {
        AudioDeviceStop(gAggregateDeviceID, gIOProcID);
        AudioDeviceDestroyIOProcID(gAggregateDeviceID, gIOProcID);
        gIOProcID = NULL;
    }
    if (gAggregateDeviceID != kAudioObjectUnknown) {
        AudioHardwareDestroyAggregateDevice(gAggregateDeviceID);
        gAggregateDeviceID = kAudioObjectUnknown;
    }
    if (gTapID != kAudioObjectUnknown) {
        AudioHardwareDestroyProcessTap(gTapID);
        gTapID = kAudioObjectUnknown;
    }
    gTapDesc = nil;
    gCallback = NULL;
    gUserData = NULL;
}

int catap_start(LacoCATapCallback callback, void* user_data) {
    if (!isAtLeastMacOS14_2()) {
        gLastDiagnostic = @"macOS version < 14.2, CATap not available";
        return -100;
    }
    __block int result = 0;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            logMacOSVersion();
            NSOperatingSystemVersion v = [[NSProcessInfo processInfo] operatingSystemVersion];

            if (gTapID != kAudioObjectUnknown) {
                gLastDiagnostic = @"CATap already active (gTapID != kAudioObjectUnknown)";
                NSLog(@"[Laconote CATap] Start rejected: tap already active, tapID=%u", (unsigned)gTapID);
                result = -1;
                return;
            }

            gCallback = callback;
            gUserData = user_data;

            // Step 1: Create CATapDescription
            NSLog(@"[Laconote CATap] Step 1/5: Creating CATapDescription (stereo global tap, private, unmuted)");
            gTapDesc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
            gTapDesc.name = @"LaconoteCATap";
            gTapDesc.privateTap = YES;
            gTapDesc.muteBehavior = CATapUnmuted;

            // Step 2: Create Process Tap
            NSLog(@"[Laconote CATap] Step 2/5: Calling AudioHardwareCreateProcessTap...");
            OSStatus status = AudioHardwareCreateProcessTap(gTapDesc, &gTapID);
            if (status != noErr) {
                gLastDiagnostic = [NSString stringWithFormat:
                    @"AudioHardwareCreateProcessTap failed: %@ on macOS %ld.%ld.%ld. "
                    @"This usually means System Audio Recording permission is not granted. "
                    @"Open System Settings → Privacy & Security → Screen & System Audio Recording.",
                    describeOSStatus(status), (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion];
                NSLog(@"[Laconote CATap] Step 2 FAILED: %@", gLastDiagnostic);
                catap_cleanup_locked();
                result = (int)status;
                return;
            }
            NSLog(@"[Laconote CATap] Step 2/5: Process tap created, tapID=%u", (unsigned)gTapID);

            // Step 3: Get tap UUID and create aggregate device
            NSString *tapUID = [[gTapDesc UUID] UUIDString];
            NSLog(@"[Laconote CATap] Step 3/5: Creating aggregate device with tap UUID=%@", tapUID);

            NSArray *taps = @[@{
                @kAudioSubTapUIDKey: tapUID,
                @kAudioSubTapDriftCompensationKey: @YES,
            }];
            NSDictionary *aggProps = @{
                @kAudioAggregateDeviceNameKey: @"LaconoteAggregateDevice",
                @kAudioAggregateDeviceUIDKey: @"com.laconote.aggregate",
                @kAudioAggregateDeviceTapListKey: taps,
                @kAudioAggregateDeviceTapAutoStartKey: @NO,
                @kAudioAggregateDeviceIsPrivateKey: @YES,
            };

            status = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)aggProps, &gAggregateDeviceID);
            if (status != noErr) {
                gLastDiagnostic = [NSString stringWithFormat:
                    @"AudioHardwareCreateAggregateDevice failed: %@ on macOS %ld.%ld.%ld",
                    describeOSStatus(status), (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion];
                NSLog(@"[Laconote CATap] Step 3 FAILED: %@", gLastDiagnostic);
                catap_cleanup_locked();
                result = (int)status;
                return;
            }
            NSLog(@"[Laconote CATap] Step 3/5: Aggregate device created, deviceID=%u", (unsigned)gAggregateDeviceID);

            // Read actual sample rate from aggregate device
            {
                Float64 nominalSR = 0;
                UInt32 srSize = sizeof(nominalSR);
                AudioObjectPropertyAddress srAddr = {
                    kAudioDevicePropertyNominalSampleRate,
                    kAudioObjectPropertyScopeGlobal,
                    kAudioObjectPropertyElementMain
                };
                OSStatus srStatus = AudioObjectGetPropertyData(gAggregateDeviceID, &srAddr, 0, NULL, &srSize, &nominalSR);
                if (srStatus == noErr && nominalSR > 0) {
                    gSampleRate = (uint32_t)nominalSR;
                    NSLog(@"[Laconote CATap] Aggregate device sample rate: %.0f Hz", nominalSR);
                } else {
                    gSampleRate = 48000;
                    NSLog(@"[Laconote CATap] Could not read aggregate device sample rate (%@), defaulting to 48000",
                          describeOSStatus(srStatus));
                }
            }

            // Step 4: Create IOProc on aggregate device
            NSLog(@"[Laconote CATap] Step 4/5: Creating IOProc on aggregate device...");
            status = AudioDeviceCreateIOProcID(gAggregateDeviceID, ioProc, NULL, &gIOProcID);
            if (status != noErr) {
                gLastDiagnostic = [NSString stringWithFormat:
                    @"AudioDeviceCreateIOProcID failed: %@ (aggregateDeviceID=%u) on macOS %ld.%ld.%ld",
                    describeOSStatus(status), (unsigned)gAggregateDeviceID,
                    (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion];
                NSLog(@"[Laconote CATap] Step 4 FAILED: %@", gLastDiagnostic);
                catap_cleanup_locked();
                result = (int)status;
                return;
            }

            // Step 5: Start aggregate device
            NSLog(@"[Laconote CATap] Step 5/5: Starting aggregate device...");
            status = AudioDeviceStart(gAggregateDeviceID, gIOProcID);
            if (status != noErr) {
                gLastDiagnostic = [NSString stringWithFormat:
                    @"AudioDeviceStart failed: %@ (aggregateDeviceID=%u) on macOS %ld.%ld.%ld",
                    describeOSStatus(status), (unsigned)gAggregateDeviceID,
                    (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion];
                NSLog(@"[Laconote CATap] Step 5 FAILED: %@", gLastDiagnostic);
                catap_cleanup_locked();
                result = (int)status;
                return;
            }

            gLastDiagnostic = nil;
            NSLog(@"[Laconote CATap] All 5 steps succeeded — tapID=%u, aggregateDeviceID=%u, audio capture active",
                  (unsigned)gTapID, (unsigned)gAggregateDeviceID);
            result = 0;
        }
    });
    return result;
}

void catap_stop(void) {
    if (!isAtLeastMacOS14_2()) return;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            NSLog(@"[Laconote CATap] Stopping — tapID=%u, aggregateDeviceID=%u",
                  (unsigned)gTapID, (unsigned)gAggregateDeviceID);

            if (gIOProcID != NULL && gAggregateDeviceID != kAudioObjectUnknown) {
                OSStatus s1 = AudioDeviceStop(gAggregateDeviceID, gIOProcID);
                if (s1 != noErr) NSLog(@"[Laconote CATap] AudioDeviceStop(aggregate) failed: %@", describeOSStatus(s1));

                OSStatus s2 = AudioDeviceDestroyIOProcID(gAggregateDeviceID, gIOProcID);
                if (s2 != noErr) NSLog(@"[Laconote CATap] AudioDeviceDestroyIOProcID(aggregate) failed: %@", describeOSStatus(s2));

                gIOProcID = NULL;
            }

            if (gAggregateDeviceID != kAudioObjectUnknown) {
                OSStatus s3 = AudioHardwareDestroyAggregateDevice(gAggregateDeviceID);
                if (s3 != noErr) NSLog(@"[Laconote CATap] AudioHardwareDestroyAggregateDevice failed: %@", describeOSStatus(s3));
                gAggregateDeviceID = kAudioObjectUnknown;
            }

            if (gTapID != kAudioObjectUnknown) {
                OSStatus s4 = AudioHardwareDestroyProcessTap(gTapID);
                if (s4 != noErr) NSLog(@"[Laconote CATap] AudioHardwareDestroyProcessTap failed: %@", describeOSStatus(s4));
                gTapID = kAudioObjectUnknown;
            }

            gTapDesc = nil;
            gCallback = NULL;
            gUserData = NULL;
            NSLog(@"[Laconote CATap] Stopped successfully");
        }
    });
}

int catap_is_available(void) {
    return isAtLeastMacOS14_2() ? 1 : 0;
}

uint32_t catap_sample_rate(void) {
    return gSampleRate;
}

int catap_probe_permission(void) {
    if (!isAtLeastMacOS14_2()) return 0;
    __block int result = 0;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            if (gTapID != kAudioObjectUnknown) {
                NSLog(@"[Laconote CATap] Permission probe: tap already active, returning granted");
                result = 1;
                return;
            }
            CATapDescription *desc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
            desc.name = @"LaconotePermissionProbe";
            desc.privateTap = YES;
            desc.muteBehavior = CATapUnmuted;
            AudioObjectID tapID = kAudioObjectUnknown;
            OSStatus status = AudioHardwareCreateProcessTap(desc, &tapID);
            if (status == noErr) {
                AudioHardwareDestroyProcessTap(tapID);
                usleep(50000); // 50ms — let Core Audio fully release the tap
                NSLog(@"[Laconote CATap] Permission probe: succeeded (tap created and destroyed)");
                result = 1;
            } else {
                gLastDiagnostic = [NSString stringWithFormat:
                    @"Permission probe failed: %@", describeOSStatus(status)];
                NSLog(@"[Laconote CATap] Permission probe: FAILED — %@", gLastDiagnostic);
                result = 0;
            }
        }
    });
    return result;
}

void catap_macos_version(uint64_t* major, uint64_t* minor, uint64_t* patch) {
    NSOperatingSystemVersion v = [[NSProcessInfo processInfo] operatingSystemVersion];
    if (major) *major = (uint64_t)v.majorVersion;
    if (minor) *minor = (uint64_t)v.minorVersion;
    if (patch) *patch = (uint64_t)v.patchVersion;
}

// Forward declaration (defined below)
int request_mic_authorization_sync(void);

/// Functional probe: actually tries to open the default mic input.
/// Returns 1 if the microphone can be accessed, 0 otherwise.
/// This is reliable regardless of TCC cache / code-signing mismatches.
static int probe_mic_access(void) {
    AVCaptureDevice *mic = [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeAudio];
    if (!mic) {
        NSLog(@"[Laconote Mic] probe_mic_access: no default audio capture device found");
        return 0;
    }
    NSError *error = nil;
    AVCaptureDeviceInput *input = [AVCaptureDeviceInput deviceInputWithDevice:mic error:&error];
    if (input && !error) {
        NSLog(@"[Laconote Mic] probe_mic_access: functional probe succeeded (device=%@)", mic.localizedName);
        return 1;
    }
    NSLog(@"[Laconote Mic] probe_mic_access: functional probe failed (device=%@, error=%@)", mic.localizedName, error);
    return 0;
}

/// Returns 1 if microphone access is authorized, 0 otherwise.
/// Uses AVAudioApplication (macOS 14+) which correctly reflects the Microphone
/// privacy toggle, unlike the legacy AVCaptureDevice API on macOS 15+.
/// Falls back to a functional probe when the API reports Denied or Undetermined
/// (the TCC status can be stale or unreliable after rebuilds).
int check_mic_authorization(void) {
    AVAudioApplication *audioApp = [AVAudioApplication sharedInstance];
    AVAudioApplicationRecordPermission perm = [audioApp recordPermission];

    NSLog(@"[Laconote Mic] check_mic_authorization: AVAudioApplication.recordPermission=%ld", (long)perm);

    if (perm == AVAudioApplicationRecordPermissionGranted) {
        return 1;
    }
    int probeResult = probe_mic_access();
    NSLog(@"[Laconote Mic] check_mic_authorization: recordPermission=%ld, probeResult=%d → returning %d",
          (long)perm, probeResult, probeResult);
    return probeResult;
}

/// Opens a URL using NSWorkspace (proper macOS API, no subprocess spawning).
/// Returns 1 on success, 0 on failure.
int open_url_nsworkspace(const char* url_cstr) {
    if (url_cstr == NULL) return 0;

    NSString* urlStr = [NSString stringWithUTF8String:url_cstr];
    if (urlStr == nil) {
        NSLog(@"[Laconote Permissions] Failed to create NSString from URL C string");
        return 0;
    }

    NSURL* url = [NSURL URLWithString:urlStr];
    if (url == nil) {
        NSLog(@"[Laconote Permissions] Invalid settings URL: %@", urlStr);
        return 0;
    }

    BOOL opened = [[NSWorkspace sharedWorkspace] openURL:url];
    NSLog(@"[Laconote Permissions] openURL %@ -> %@", urlStr, opened ? @"success" : @"failed");
    return opened ? 1 : 0;
}

/// Requests microphone access and blocks until the user responds or a
/// 30-second timeout elapses. Uses AVCaptureDevice API which triggers
/// the macOS TCC permission dialog when status is undetermined.
/// The caller is responsible for activation policy management.
/// Returns 1 if granted, 0 otherwise.
int request_mic_authorization_sync(void) {
    AVAudioApplication *audioApp = [AVAudioApplication sharedInstance];
    AVAudioApplicationRecordPermission perm = [audioApp recordPermission];
    NSLog(@"[Laconote Mic] request_mic_authorization_sync: recordPermission=%ld", (long)perm);

    if (perm == AVAudioApplicationRecordPermissionGranted) {
        return 1;
    }
    if (perm == AVAudioApplicationRecordPermissionDenied) {
        NSLog(@"[Laconote Mic] Permission denied, user must enable in System Settings.");
        return 0;
    }
    NSLog(@"[Laconote Mic] Permission undetermined, requesting access via AVCaptureDevice...");

    dispatch_semaphore_t sema = dispatch_semaphore_create(0);
    __block BOOL granted = NO;
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL g) {
        NSLog(@"[Laconote Mic] requestAccess completionHandler: granted=%d", g);
        granted = g;
        dispatch_semaphore_signal(sema);
    }];
    dispatch_semaphore_wait(sema, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC));

    NSLog(@"[Laconote Mic] request_mic_authorization_sync result: granted=%d", granted);
    return granted ? 1 : 0;
}

/// Switches the app to Accessory activation policy (hides dock icon).
void set_accessory_policy(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        NSLog(@"[Laconote] Switched to Accessory activation policy (dock icon hidden)");
    });
}

/// Switches the app to Regular activation policy (shows dock icon, can come to foreground).
/// Called before showing a window so the app can properly activate.
void set_regular_policy(void) {
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    NSLog(@"[Laconote] Switched to Regular activation policy (app can activate)");
}

void request_mic_then_set_accessory(void) {
    AVAudioApplication *audioApp = [AVAudioApplication sharedInstance];
    AVAudioApplicationRecordPermission perm = [audioApp recordPermission];
    NSLog(@"[Laconote Mic] request_mic_then_set_accessory: recordPermission=%ld", (long)perm);

    if (perm == AVAudioApplicationRecordPermissionGranted ||
        perm == AVAudioApplicationRecordPermissionDenied) {
        NSLog(@"[Laconote Mic] Permission already resolved (%ld), setting accessory policy immediately",
              (long)perm);
        set_accessory_policy();
        return;
    }

    NSLog(@"[Laconote Mic] Permission undetermined, requesting access before hiding dock icon...");
    dispatch_async(dispatch_get_main_queue(), ^{
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL granted) {
            NSLog(@"[Laconote Mic] requestAccess completionHandler: granted=%d, now hiding dock icon", granted);
            set_accessory_policy();
        }];
    });
}
