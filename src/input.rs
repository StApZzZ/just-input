use std::{error::Error, fmt, thread, time::Duration};

use crate::text::prepare_text;

#[derive(Debug)]
#[allow(dead_code)]
pub enum TypeError {
    UnsupportedPlatform,
    SendFailed(std::io::Error),
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => write!(f, "typing is only implemented on Windows"),
            Self::SendFailed(error) => write!(f, "failed to send keyboard input: {error}"),
        }
    }
}

impl Error for TypeError {}

pub fn type_text(text: &str, interval: Duration) -> Result<(), TypeError> {
    for unit in prepare_text(text) {
        platform::send_unit(&unit)?;

        if !interval.is_zero() {
            thread::sleep(interval);
        }
    }

    Ok(())
}

#[cfg(windows)]
mod platform {
    use std::mem::size_of;

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        KEYEVENTF_UNICODE, VK_RETURN,
    };

    use crate::text::InputUnit;

    use super::TypeError;

    pub fn send_unit(unit: &InputUnit) -> Result<(), TypeError> {
        match unit {
            InputUnit::Unicode(code_units) => {
                for code_unit in code_units {
                    send_unicode(*code_unit)?;
                }
            }
            InputUnit::Enter => send_enter()?,
        }

        Ok(())
    }

    fn send_unicode(code_unit: u16) -> Result<(), TypeError> {
        let mut inputs = [
            keyboard_input(0, code_unit, KEYEVENTF_UNICODE),
            keyboard_input(0, code_unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP),
        ];

        send_inputs(&mut inputs)
    }

    fn send_enter() -> Result<(), TypeError> {
        let mut inputs = [
            keyboard_input(VK_RETURN, 0, 0),
            keyboard_input(VK_RETURN, 0, KEYEVENTF_KEYUP),
        ];

        send_inputs(&mut inputs)
    }

    fn keyboard_input(vk: u16, scan: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn send_inputs(inputs: &mut [INPUT]) -> Result<(), TypeError> {
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            )
        };

        if sent == inputs.len() as u32 {
            Ok(())
        } else {
            Err(TypeError::SendFailed(std::io::Error::last_os_error()))
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use crate::text::InputUnit;

    use super::TypeError;

    pub fn send_unit(_unit: &InputUnit) -> Result<(), TypeError> {
        Err(TypeError::UnsupportedPlatform)
    }
}
