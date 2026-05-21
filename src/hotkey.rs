use std::{error::Error, fmt, sync::mpsc::Sender};

pub const HOTKEY_LABEL: &str = "Ctrl+Alt+J";

#[derive(Debug)]
#[allow(dead_code)]
pub enum HotkeyError {
    UnsupportedPlatform,
    RegisterFailed(std::io::Error),
    StartupFailed,
}

impl fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => write!(f, "global hotkey is only implemented on Windows"),
            Self::RegisterFailed(error) => write!(f, "failed to register hotkey: {error}"),
            Self::StartupFailed => write!(f, "hotkey thread did not report startup status"),
        }
    }
}

impl Error for HotkeyError {}

pub struct HotkeyHandle {
    _inner: platform::HotkeyHandle,
}

impl HotkeyHandle {
    pub fn register(trigger_tx: Sender<()>) -> Result<Self, HotkeyError> {
        platform::HotkeyHandle::register(trigger_tx).map(|inner| Self { _inner: inner })
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        ptr::null_mut,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Sender},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Duration,
    };

    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::{
            RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
        },
        WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY},
    };

    use super::HotkeyError;

    const HOTKEY_ID: i32 = 1;
    const VK_J: u32 = b'J' as u32;

    pub struct HotkeyHandle {
        stop: Arc<AtomicBool>,
        join: Option<JoinHandle<()>>,
    }

    impl HotkeyHandle {
        pub fn register(trigger_tx: Sender<()>) -> Result<Self, HotkeyError> {
            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = Arc::clone(&stop);
            let (ready_tx, ready_rx) = mpsc::channel();

            let join = thread::spawn(move || {
                let modifiers = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
                let registered = unsafe { RegisterHotKey(null_mut(), HOTKEY_ID, modifiers, VK_J) };

                if registered == 0 {
                    let _ = ready_tx.send(Err(HotkeyError::RegisterFailed(
                        std::io::Error::last_os_error(),
                    )));
                    return;
                }

                let _ = ready_tx.send(Ok(()));

                while !thread_stop.load(Ordering::Relaxed) {
                    let mut message: MSG = unsafe { std::mem::zeroed() };

                    while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
                        if message.message == WM_HOTKEY && message.wParam == HOTKEY_ID as usize {
                            let _ = trigger_tx.send(());
                        }
                    }

                    thread::sleep(Duration::from_millis(10));
                }

                unsafe {
                    UnregisterHotKey(null_mut(), HOTKEY_ID);
                }
            });

            match ready_rx.recv_timeout(Duration::from_secs(2)) {
                Ok(Ok(())) => Ok(Self {
                    stop,
                    join: Some(join),
                }),
                Ok(Err(error)) => {
                    let _ = join.join();
                    Err(error)
                }
                Err(_) => {
                    stop.store(true, Ordering::Relaxed);
                    let _ = join.join();
                    Err(HotkeyError::StartupFailed)
                }
            }
        }
    }

    impl Drop for HotkeyHandle {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);

            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::sync::mpsc::Sender;

    use super::HotkeyError;

    pub struct HotkeyHandle;

    impl HotkeyHandle {
        pub fn register(_trigger_tx: Sender<()>) -> Result<Self, HotkeyError> {
            Err(HotkeyError::UnsupportedPlatform)
        }
    }
}
