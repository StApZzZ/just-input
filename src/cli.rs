use std::{ffi::OsString, fs, path::PathBuf, time::Duration};

#[derive(Debug)]
pub enum CliAction {
    Gui,
    Help,
    Type(TypeOptions),
}

#[derive(Debug)]
pub struct TypeOptions {
    pub text: String,
    pub delay: Duration,
    pub interval: Duration,
}

pub fn parse_args<I>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter().peekable();

    if args.peek().is_none() {
        return Ok(CliAction::Gui);
    }

    let mut text: Option<String> = None;
    let mut file: Option<PathBuf> = None;
    let mut delay = Duration::from_secs(3);
    let mut interval = Duration::from_millis(10);

    while let Some(arg) = args.next() {
        let arg = arg
            .into_string()
            .map_err(|_| "arguments must be valid Unicode".to_owned())?;

        match arg.as_str() {
            "-h" | "--help" => return Ok(CliAction::Help),
            "--text" => {
                text = Some(next_value(&mut args, "--text")?);
            }
            "--file" => {
                file = Some(PathBuf::from(next_value(&mut args, "--file")?));
            }
            "--delay" => {
                delay = Duration::from_secs_f32(parse_f32(&mut args, "--delay")?);
            }
            "--interval-ms" => {
                interval = Duration::from_millis(parse_u64(&mut args, "--interval-ms")?);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    match (text, file) {
        (Some(_), Some(_)) => Err("--text and --file cannot be used together".to_owned()),
        (Some(text), None) => Ok(CliAction::Type(TypeOptions {
            text,
            delay,
            interval,
        })),
        (None, Some(file)) => Ok(CliAction::Type(TypeOptions {
            text: fs::read_to_string(&file)
                .map_err(|error| format!("failed to read {}: {error}", file.display()))?,
            delay,
            interval,
        })),
        (None, None) => Err("CLI mode needs --text or --file; omit arguments for GUI".to_owned()),
    }
}

pub fn help_text() -> &'static str {
    "Just Input\n\n\
Usage:\n\
  just-input.exe\n\
  just-input.exe --text \"hello\" --delay 3\n\
  just-input.exe --file input.txt --delay 3\n\n\
Options:\n\
  --text <TEXT>          Text to type after the delay\n\
  --file <PATH>          UTF-8 file to type after the delay\n\
  --delay <SECONDS>      Delay before typing, default 3\n\
  --interval-ms <MS>     Pause between characters, default 10\n\
  -h, --help             Show this help\n"
}

fn next_value<I>(args: &mut I, name: &str) -> Result<String, String>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .ok_or_else(|| format!("{name} needs a value"))?
        .into_string()
        .map_err(|_| format!("{name} value must be valid Unicode"))
}

fn parse_f32<I>(args: &mut I, name: &str) -> Result<f32, String>
where
    I: Iterator<Item = OsString>,
{
    let value = next_value(args, name)?;
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("{name} must be a number"))?;

    if !parsed.is_finite() || parsed.is_sign_negative() {
        Err(format!("{name} must be a finite non-negative number"))
    } else {
        Ok(parsed)
    }
}

fn parse_u64<I>(args: &mut I, name: &str) -> Result<u64, String>
where
    I: Iterator<Item = OsString>,
{
    next_value(args, name)?
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::{parse_args, CliAction};
    use std::{ffi::OsString, time::Duration};

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_args_opens_gui() {
        assert!(matches!(parse_args(args(&[])).unwrap(), CliAction::Gui));
    }

    #[test]
    fn parses_text_mode() {
        let action = parse_args(args(&[
            "--text",
            "hello",
            "--delay",
            "1.5",
            "--interval-ms",
            "25",
        ]))
        .unwrap();

        match action {
            CliAction::Type(options) => {
                assert_eq!(options.text, "hello");
                assert_eq!(options.delay, Duration::from_secs_f32(1.5));
                assert_eq!(options.interval, Duration::from_millis(25));
            }
            _ => panic!("expected type action"),
        }
    }

    #[test]
    fn rejects_text_and_file_together() {
        assert!(parse_args(args(&["--text", "hello", "--file", "input.txt"])).is_err());
    }
}
