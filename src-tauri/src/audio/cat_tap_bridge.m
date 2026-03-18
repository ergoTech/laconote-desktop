#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>
#import <CoreAudio/CATapDescription.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/AudioHardware.h>
#import <AppKit/AppKit.h>
#include <string.h>
#include <stdlib.h>

typedef void (*LacoCATapCallback)(const float* samples, int count, int channels, void* user_data);

static CATapDescription *gTapDesc = nil;
static AudioObjectID gTapID = kAudioObjectUnknown;
static AudioDeviceIOProcID gIOProcID = NULL;
static LacoCATapCallback gCallback = NULL;
static void* gUserData = NULL;

static dispatch_queue_t gTapQueue = NULL;
static BOOL gVersionLogged = NO;

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

int catap_start(LacoCATapCallback callback, void* user_data) {
    if (!isAtLeastMacOS14_2()) {
        return -100;
    }
    __block int result = 0;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            logMacOSVersion();

            if (gTapID != kAudioObjectUnknown) { result = -1; return; }

            gCallback = callback;
            gUserData = user_data;

            gTapDesc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
            gTapDesc.name = @"LaconoteCATap";
            gTapDesc.privateTap = YES;
            gTapDesc.muteBehavior = CATapUnmuted;

            OSStatus status = AudioHardwareCreateProcessTap(gTapDesc, &gTapID);
            if (status != noErr) {
                NSOperatingSystemVersion v = [[NSProcessInfo processInfo] operatingSystemVersion];
                NSLog(@"[Laconote CATap] AudioHardwareCreateProcessTap failed: %d on macOS %ld.%ld.%ld — check System Audio permission in System Settings → Privacy & Security → Screen Recording",
                      (int)status, (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion);
                gTapDesc = nil;
                gCallback = NULL;
                gUserData = NULL;
                result = (int)status;
                return;
            }

            status = AudioDeviceCreateIOProcID(gTapID, ioProc, NULL, &gIOProcID);
            if (status != noErr) {
                NSLog(@"[Laconote CATap] AudioDeviceCreateIOProcID failed: %d", (int)status);
                AudioHardwareDestroyProcessTap(gTapID);
                gTapID = kAudioObjectUnknown;
                gTapDesc = nil;
                gCallback = NULL;
                gUserData = NULL;
                result = (int)status;
                return;
            }

            status = AudioDeviceStart(gTapID, gIOProcID);
            if (status != noErr) {
                NSLog(@"[Laconote CATap] AudioDeviceStart failed: %d", (int)status);
                AudioDeviceDestroyIOProcID(gTapID, gIOProcID);
                gIOProcID = NULL;
                AudioHardwareDestroyProcessTap(gTapID);
                gTapID = kAudioObjectUnknown;
                gTapDesc = nil;
                gCallback = NULL;
                gUserData = NULL;
                result = (int)status;
                return;
            }

            NSLog(@"[Laconote CATap] Started successfully, tapID=%u", (unsigned)gTapID);
            result = 0;
        }
    });
    return result;
}

void catap_stop(void) {
    if (!isAtLeastMacOS14_2()) return;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            if (gTapID != kAudioObjectUnknown) {
                if (gIOProcID != NULL) {
                    OSStatus s1 = AudioDeviceStop(gTapID, gIOProcID);
                    if (s1 != noErr) NSLog(@"[Laconote CATap] AudioDeviceStop failed: %d", (int)s1);

                    OSStatus s2 = AudioDeviceDestroyIOProcID(gTapID, gIOProcID);
                    if (s2 != noErr) NSLog(@"[Laconote CATap] AudioDeviceDestroyIOProcID failed: %d", (int)s2);

                    gIOProcID = NULL;
                }
                OSStatus s3 = AudioHardwareDestroyProcessTap(gTapID);
                if (s3 != noErr) NSLog(@"[Laconote CATap] AudioHardwareDestroyProcessTap failed: %d", (int)s3);

                NSLog(@"[Laconote CATap] Stopped, tapID=%u", (unsigned)gTapID);
                gTapID = kAudioObjectUnknown;
            }
            gTapDesc = nil;
            gCallback = NULL;
            gUserData = NULL;
        }
    });
}

int catap_is_available(void) {
    return isAtLeastMacOS14_2() ? 1 : 0;
}

int catap_probe_permission(void) {
    if (!isAtLeastMacOS14_2()) return 0;
    __block int result = 0;
    dispatch_sync(gTapQueue, ^{
        @autoreleasepool {
            if (gTapID != kAudioObjectUnknown) {
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
                result = 1;
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

/// Returns 1 if microphone access is authorized, 0 otherwise.
/// Uses AVCaptureDevice.authorizationStatus which reads the cached TCC state
/// without opening any audio streams or triggering new TCC IPC requests.
int check_mic_authorization(void) {
    AVAuthorizationStatus status = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
    return (status == AVAuthorizationStatusAuthorized) ? 1 : 0;
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

/// Requests microphone access via AVFoundation and blocks until the user responds.
/// Returns 1 if granted, 0 otherwise.
int request_mic_authorization_sync(void) {
    AVAuthorizationStatus current = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
    if (current == AVAuthorizationStatusAuthorized) {
        return 1;
    }
    if (current == AVAuthorizationStatusDenied || current == AVAuthorizationStatusRestricted) {
        return 0;
    }
    dispatch_semaphore_t sema = dispatch_semaphore_create(0);
    __block BOOL granted = NO;
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL g) {
        granted = g;
        dispatch_semaphore_signal(sema);
    }];
    dispatch_semaphore_wait(sema, DISPATCH_TIME_FOREVER);
    return granted ? 1 : 0;
}
