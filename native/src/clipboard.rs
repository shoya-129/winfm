#![cfg(windows)]

use std::slice;

use windows::Win32::{
    Foundation::{HANDLE, HGLOBAL},
    System::{
        DataExchange::{
            CloseClipboard,
            EmptyClipboard,
            GetClipboardData,
            IsClipboardFormatAvailable,
            OpenClipboard,
            SetClipboardData,
        },
        Memory::{
            GlobalAlloc,
            GlobalLock,
            GlobalUnlock,
            GMEM_MOVEABLE,
            GMEM_ZEROINIT,
        },
    },
};

const CF_UNICODETEXT: u32 = 13;

/// Provides access to the Windows system clipboard.
///
/// Windows owns the actual clipboard contents.
/// This object is only a lightweight interface to it.
pub struct Clipboard {
    _state: u8,
}

impl Clipboard {
    /// Creates a new Windows clipboard interface.
    pub fn init() -> Clipboard {
        Clipboard {
            _state: 0,
        }
    }

    /// Returns the current Unicode text from the Windows clipboard.
    ///
    /// Returns an empty string when the clipboard cannot be opened,
    /// does not contain Unicode text, or the clipboard data cannot
    /// be read.
    pub fn get(&self) -> String {
        unsafe {
            if OpenClipboard(None).is_err() {
                return String::new();
            }

            let result = Self::get_inner();

            let _ = CloseClipboard();

            result
        }
    }

    /// Replaces the Windows clipboard contents with Unicode text.
    ///
    /// Returns `true` when the text was successfully placed
    /// on the clipboard.
    pub fn set(&self, text: String) -> bool {
        unsafe {
            if OpenClipboard(None).is_err() {
                return false;
            }

            let result = Self::set_inner(&text);

            let _ = CloseClipboard();

            result
        }
    }

    /// Clears the contents of the Windows clipboard.
    ///
    /// Returns `true` when the clipboard was successfully cleared.
    pub fn clear(&self) -> bool {
        unsafe {
            if OpenClipboard(None).is_err() {
                return false;
            }

            let result = EmptyClipboard().is_ok();

            let _ = CloseClipboard();

            result
        }
    }

    unsafe fn get_inner() -> String {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return String::new();
        }

        let handle = match GetClipboardData(CF_UNICODETEXT) {
            Ok(handle) => handle,
            Err(_) => return String::new(),
        };

        let hglobal = HGLOBAL(handle.0);

        let memory = GlobalLock(hglobal);

        if memory.is_null() {
            return String::new();
        }

        let ptr = memory as *const u16;

        let mut length = 0usize;

        while *ptr.add(length) != 0 {
            length += 1;
        }

        let text = String::from_utf16_lossy(
            slice::from_raw_parts(ptr, length),
        );

        let _ = GlobalUnlock(hglobal);

        text
    }

    unsafe fn set_inner(text: &str) -> bool {
        if EmptyClipboard().is_err() {
            return false;
        }

        let mut utf16: Vec<u16> = text.encode_utf16().collect();

        // CF_UNICODETEXT requires a terminating NUL.
        utf16.push(0);

        let bytes = utf16.len() * std::mem::size_of::<u16>();

        let memory = match GlobalAlloc(
            GMEM_MOVEABLE | GMEM_ZEROINIT,
            bytes,
        ) {
            Ok(memory) => memory,
            Err(_) => return false,
        };

        let ptr = GlobalLock(memory);

        if ptr.is_null() {
            return false;
        }

        std::ptr::copy_nonoverlapping(
            utf16.as_ptr() as *const u8,
            ptr as *mut u8,
            bytes,
        );

        let _ = GlobalUnlock(memory);

        // Windows owns this memory after successful SetClipboardData.
        let handle = HANDLE(memory.0);

        SetClipboardData(
            CF_UNICODETEXT,
            Some(handle),
        )
        .is_ok()
    }
}