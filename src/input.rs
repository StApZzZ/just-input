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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    EnUs,
    RuQwerty,
}

pub fn type_text(text: &str, interval: Duration, layout: Layout) -> Result<(), TypeError> {
    for unit in prepare_text(text) {
        platform::send_unit(&unit, layout)?;

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
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA,
        VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_RETURN, VK_SHIFT, VK_SPACE,
    };

    use crate::text::InputUnit;

    use super::TypeError;

    pub fn send_unit(unit: &InputUnit, layout: super::Layout) -> Result<(), TypeError> {
        match unit {
            InputUnit::Unicode(code_units) => {
                if let [cu] = code_units.as_slice() {
                    if *cu < 128 {
                        let ch = *cu as u8 as char;
                        if let Some((vk, shift)) = ascii_to_vk(ch, layout) {
                            return send_vk_char(vk, shift);
                        }
                    }
                }
                for cu in code_units {
                    send_unicode(*cu)?;
                }
            }
            InputUnit::Enter => {
                send_enter()?;
            }
        }

        Ok(())
    }

    fn send_vk_char(vk: u16, shift: bool) -> Result<(), TypeError> {
        if shift {
            let mut inputs = [
                keyboard_input(VK_SHIFT, 0, 0),
                keyboard_input(vk, 0, 0),
                keyboard_input(vk, 0, KEYEVENTF_KEYUP),
                keyboard_input(VK_SHIFT, 0, KEYEVENTF_KEYUP),
            ];
            send_inputs(&mut inputs)
        } else {
            let mut inputs = [
                keyboard_input(vk, 0, 0),
                keyboard_input(vk, 0, KEYEVENTF_KEYUP),
            ];
            send_inputs(&mut inputs)
        }
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

    fn ascii_to_vk(ch: char, layout: super::Layout) -> Option<(u16, bool)> {
        match layout {
            super::Layout::EnUs => ascii_to_vk_en(ch),
            super::Layout::RuQwerty => ascii_to_vk_ru(ch),
        }
    }

    fn ascii_to_vk_en(ch: char) -> Option<(u16, bool)> {
        Some(match ch {
            ' ' => (VK_SPACE, false),
            '!' => (0x31, true),
            '"' => (VK_OEM_7, true),
            '#' => (0x33, true),
            '$' => (0x34, true),
            '%' => (0x35, true),
            '&' => (0x37, true),
            '\'' => (VK_OEM_7, false),
            '(' => (0x39, true),
            ')' => (0x30, true),
            '*' => (0x38, true),
            '+' => (VK_OEM_PLUS, true),
            ',' => (VK_OEM_COMMA, false),
            '-' => (VK_OEM_MINUS, false),
            '.' => (VK_OEM_PERIOD, false),
            '/' => (VK_OEM_2, false),
            '0'..='9' => (ch as u16, false),
            ':' => (VK_OEM_1, true),
            ';' => (VK_OEM_1, false),
            '<' => (VK_OEM_COMMA, true),
            '=' => (VK_OEM_PLUS, false),
            '>' => (VK_OEM_PERIOD, true),
            '?' => (VK_OEM_2, true),
            '@' => (0x32, true),
            'A'..='Z' => (ch as u16, true),
            '[' => (VK_OEM_4, false),
            '\\' => (VK_OEM_5, false),
            ']' => (VK_OEM_6, false),
            '^' => (0x36, true),
            '_' => (VK_OEM_MINUS, true),
            '`' => (VK_OEM_3, false),
            'a'..='z' => (ch as u16 - 0x20, false),
            '{' => (VK_OEM_4, true),
            '|' => (VK_OEM_5, true),
            '}' => (VK_OEM_6, true),
            '~' => (VK_OEM_3, true),
            _ => return None,
        })
    }

    fn ascii_to_vk_ru(ch: char) -> Option<(u16, bool)> {
        Some(match ch {
            // Same physical key + modifier as EN-US
            ' ' => (VK_SPACE, false),
            '!' => (0x31, true),
            '%' => (0x35, true),
            '(' => (0x39, true),
            ')' => (0x30, true),
            '*' => (0x38, true),
            '-' => (VK_OEM_MINUS, false),
            '_' => (VK_OEM_MINUS, true),
            '=' => (VK_OEM_PLUS, false),
            '+' => (VK_OEM_PLUS, true),
            '\\' => (VK_OEM_5, false),
            '0'..='9' => (ch as u16, false),
            'A'..='Z' => (ch as u16, true),
            'a'..='z' => (ch as u16 - 0x20, false),
            // Different from EN-US
            '"' => (0x32, true),      // Shift+2
            ';' => (0x34, true),      // Shift+4
            ':' => (0x36, true),      // Shift+6
            '?' => (0x37, true),      // Shift+7
            '.' => (VK_OEM_2, false), // / key = . on RU
            ',' => (VK_OEM_2, true),  // Shift+/ = , on RU
            '/' => (VK_OEM_5, true),  // Shift+\ = / on RU
            // Not on standard RU layout → Unicode fallback
            _ => return None,
        })
    }
}

#[cfg(not(windows))]
mod platform {
    use crate::text::InputUnit;

    use super::TypeError;

    pub fn send_unit(_unit: &InputUnit, _layout: super::Layout) -> Result<(), TypeError> {
        Err(TypeError::UnsupportedPlatform)
    }
}
