#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputUnit {
    Unicode(Vec<u16>),
    Enter,
}

pub fn prepare_text(text: &str) -> Vec<InputUnit> {
    let mut units = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                units.push(InputUnit::Enter);
            }
            '\n' => units.push(InputUnit::Enter),
            _ => {
                let mut buf = [0_u16; 2];
                units.push(InputUnit::Unicode(ch.encode_utf16(&mut buf).to_vec()));
            }
        }
    }

    units
}

#[cfg(test)]
mod tests {
    use super::{prepare_text, InputUnit};

    fn unicode(ch: char) -> InputUnit {
        let mut buf = [0_u16; 2];
        InputUnit::Unicode(ch.encode_utf16(&mut buf).to_vec())
    }

    #[test]
    fn prepares_empty_text() {
        assert_eq!(prepare_text(""), Vec::<InputUnit>::new());
    }

    #[test]
    fn normalizes_newlines_to_enter() {
        assert_eq!(
            prepare_text("a\r\nb\nc\rd"),
            vec![
                unicode('a'),
                InputUnit::Enter,
                unicode('b'),
                InputUnit::Enter,
                unicode('c'),
                InputUnit::Enter,
                unicode('d'),
            ]
        );
    }

    #[test]
    fn keeps_unicode_independent_of_keyboard_layout() {
        assert_eq!(
            prepare_text("Привет"),
            "Привет".chars().map(unicode).collect::<Vec<_>>()
        );
    }

    #[test]
    fn keeps_emoji_as_one_surrogate_pair_unit() {
        assert_eq!(
            prepare_text("🙂"),
            vec![InputUnit::Unicode(vec![0xD83D, 0xDE42])]
        );
    }
}
