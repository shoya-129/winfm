#![cfg(windows)]

use std::ffi::c_void;

use windows::{
    core::{Interface, GUID, HRESULT},
    Win32::{
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        },
    },
};

/// Provides access to the Windows master audio volume.
///
/// `Volume` is a lightweight interface to the default Windows
/// render audio endpoint.
pub struct Volume {
    _state: u8,
}

impl Volume {
    /// Creates a new Windows volume interface.
    pub fn init() -> Volume {
        Volume { _state: 0 }
    }

    /// Returns the current Windows master volume as a percentage.
    ///
    /// The returned value is between `0` and `100`.
    ///
    /// Returns `0` when the Windows audio endpoint cannot be accessed.
    pub fn percent(&self) -> u32 {
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return 0;
            }

            let result = self.percent_inner();

            CoUninitialize();

            result
        }
    }

    /// Sets the Windows master volume.
    ///
    /// The value is clamped to `0..=100`.
    ///
    /// Returns `true` when Windows successfully changes the volume.
    pub fn set(&self, percent: i64) -> bool {
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return false;
            }

            let percent = percent.clamp(0, 100) as u32;
            let result = self.set_inner(percent);

            CoUninitialize();

            result
        }
    }

    /// Returns whether the Windows master volume is muted.
    ///
    /// Returns `false` when the Windows audio endpoint cannot be accessed.
    pub fn muted(&self) -> bool {
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return false;
            }

            let result = self.muted_inner();

            CoUninitialize();

            result
        }
    }

    /// Mutes or unmutes the Windows master volume.
    ///
    /// Returns `true` when Windows successfully changes the mute state.
    pub fn set_muted(&self, muted: bool) -> bool {
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return false;
            }

            let result = self.set_muted_inner(muted);

            CoUninitialize();

            result
        }
    }

    /// Gets the default Windows render endpoint volume interface.
    unsafe fn endpoint_volume(&self) -> Option<IAudioEndpointVolume> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;

        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;

        /*
            IMMDevice::Activate is not exposed as a normal method by
            windows 0.62.2 for this interface, so call the COM vtable
            directly.

            IMMDevice vtable:
              0 = QueryInterface
              1 = AddRef
              2 = Release
              3 = Activate
        */

        type ActivateFn = unsafe extern "system" fn(
            this: *mut c_void,
            iid: *const GUID,
            clsctx: u32,
            activation_params: *const c_void,
            interface: *mut *mut c_void,
        ) -> HRESULT;

        let raw_device = device.as_raw();

        if raw_device.is_null() {
            return None;
        }

        let vtable = *(raw_device as *const *const *const c_void);

        let activate: ActivateFn = std::mem::transmute(vtable.add(3).read());

        let mut endpoint: *mut c_void = std::ptr::null_mut();

        let result = activate(
            raw_device,
            &IAudioEndpointVolume::IID,
            CLSCTX_ALL.0,
            std::ptr::null(),
            &mut endpoint,
        );

        if result.is_err() || endpoint.is_null() {
            return None;
        }

        Some(IAudioEndpointVolume::from_raw(endpoint))
    }

    /// Reads the master volume from the audio endpoint.
    unsafe fn percent_inner(&self) -> u32 {
        let endpoint = match self.endpoint_volume() {
            Some(endpoint) => endpoint,
            None => return 0,
        };

        let value = match endpoint.GetMasterVolumeLevelScalar() {
            Ok(value) => value,
            Err(_) => return 0,
        };

        (value.clamp(0.0, 1.0) * 100.0).round() as u32
    }

    /// Changes the master volume.
    unsafe fn set_inner(&self, percent: u32) -> bool {
        let endpoint = match self.endpoint_volume() {
            Some(endpoint) => endpoint,
            None => return false,
        };

        let value = (percent.min(100) as f32) / 100.0;

        endpoint
            .SetMasterVolumeLevelScalar(value, std::ptr::null())
            .is_ok()
    }

    /// Reads the current mute state.
    unsafe fn muted_inner(&self) -> bool {
        let endpoint = match self.endpoint_volume() {
            Some(endpoint) => endpoint,
            None => return false,
        };

        endpoint
            .GetMute()
            .map(|muted| muted.as_bool())
            .unwrap_or(false)
    }

    /// Changes the mute state.
    unsafe fn set_muted_inner(&self, muted: bool) -> bool {
        let endpoint = match self.endpoint_volume() {
            Some(endpoint) => endpoint,
            None => return false,
        };

        endpoint.SetMute(muted, std::ptr::null()).is_ok()
    }
}
