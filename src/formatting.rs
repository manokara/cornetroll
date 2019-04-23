use std::fmt;
use super::{
    DEFAULT_INFO_SETTINGS,
    DEFAULT_META_SETTINGS,
    DEFAULT_TIME_SETTINGS,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisplayFormat {
    Prev,
    Next,
    PlayPause,
    Status,
    /// `(show number of players, show name)`
    PlayerInfo(bool, bool),
    /// `(buffer_size, scroll_timeout)`
    Metadata(u8, u8),
    /// `(show_length, use_remaining)`
    Time(bool, bool),
    String(String),
}

#[derive(Clone, Debug)]
pub enum MetaFormat {
    Artist,
    Artists,
    Album,
    AlbumArtist,
    Title,
    Track,
    Optional(Vec<MetaFormat>),
    String(String),
}

#[derive(Debug)]
pub enum DisplayFormatError {
    Unexpected(usize, char),
    ArgumentCount(usize, String, usize, usize),
    WrongArgumentType(usize),
    InvalidArgument(usize),
    UnknownBlock(usize, String),
}

impl fmt::Display for DisplayFormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use DisplayFormatError::*;
        match self {
            Unexpected(pos, m) => write!(f, "at {}: {}", pos, m),
            ArgumentCount(pos, block, expected, got) => write!(f, "at {}: expected {} arguments for block '{}', got {}", pos, expected, block, got),
            WrongArgumentType(pos) => write!(f, "at {}: wrong argument argument", pos),
            InvalidArgument(pos) => write!(f, "at {}: invalid argument", pos),
            UnknownBlock(pos, name) => write!(f, "at {}: unknown block '{}'", pos, name),
        }
    }
}

#[derive(Debug)]
pub enum MetaFormatError {
    Unexpected(usize, char),
    UnknownBlock(usize, String),
}

impl fmt::Display for MetaFormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use MetaFormatError::*;
        match self {
            Unexpected(pos, m) => write!(f, "at {}: {}", pos, m),
            UnknownBlock(pos, name) => write!(f, "at {}: unknown block '{}'", pos, name),
        }
    }
}

pub fn process_display_format(format: &str) -> Result<Vec<DisplayFormat>, DisplayFormatError> {
    use DisplayFormatError::*;

    const BLOCKS: &[&'static str] = &[
        "prev", "next", "play-pause",
        "info", "metadata", "time",
        "status",
    ];

    #[derive(PartialEq, Eq)]
    enum State {
        Text,
        BeginBlock,
        ArgumentList,
        Argument(String),
    }

    enum Value {
        Number(u8),
        Bool(bool),
    }

    macro_rules! check_arg_count {
        ($pos:expr, $name:ident, $args:ident, $len:expr) => {
            if $args.len() != $len {
                return Err(ArgumentCount($pos, $name.to_string(), $len, $args.len()));
            }
        };

        ($pos:expr, $name:ident, $args:ident, $len:expr, g) => {
            if $args.len() > $len {
                return Err(ArgumentCount($pos, $name.to_string(), $len, $args.len()));
            }
        };
    }

    macro_rules! check_arg_type {
        ($args:ident, $type:ident) => {
            for i in 0..$args.len() {
                if let Some(v) = &$args[i] {
                    if let Value::$type(_) = v { } else {
                        return Err(WrongArgumentType(i));
                    }
                }
            }
        };
    }

    macro_rules! extract_arg {
        ($type:ident, $args:ident, $ind:expr, $default:expr) => {
            match $args.get($ind) {
                Some(v1) => {
                    match v1 {
                        Some(v2) => match v2 {
                            Value::$type(v) => *v,
                            _ => unreachable!(),
                        },

                        None => $default,
                    }
                }

                None => $default,
            }
        };
    }

    macro_rules! test_block_name {
        ($pos:expr, $name:ident) => {
            if !BLOCKS.contains(&$name.as_str()) { return Err(UnknownBlock($pos, $name.clone())); }
        };
    }

    macro_rules! flush_buffer {
        ($buffer:expr, $vec:ident) => {
            if $buffer.len() > 0 {
                $vec.push(DisplayFormat::String($buffer.clone()));
                $buffer.clear();
            }
        };
    }

    fn parse_value(pos: usize, value: &str) -> Result<Value, DisplayFormatError> {
        if let Ok(n) = value.parse::<u8>() { return Ok(Value::Number(n)) }
        else if let Ok(b) = value.parse::<bool>() { return Ok(Value::Bool(b)) }

        // Basically strings and big numbers
        Err(DisplayFormatError::InvalidArgument(pos))
    }


    fn validate_arguments(pos: usize, name: &str, args: &Vec<Option<Value>>) -> Result<(), DisplayFormatError> {
        match name {
            "prev" | "next" | "play-pause" | "status" => {
                check_arg_count!(pos, name, args, 0);
            }

            "info" => {
                check_arg_count!(pos, name, args, 2, g);
                check_arg_type!(args, Bool);
            }

            "metadata" => {
                check_arg_count!(pos, name, args, 2, g);
                check_arg_type!(args, Number);
            }

            "time" => {
                check_arg_count!(pos, name, args, 2, g);
                check_arg_type!(args, Bool);
            }

            _ => (),
        }

        Ok(())
    }

    let mut state = State::Text;
    let mut buffer = String::new();
    let mut context_pos = 0usize;
    let mut current_block = String::new();
    let mut result = Vec::<DisplayFormat>::new();
    let mut args = Vec::<Option<Value>>::new();

    for (pos, c) in format.chars().enumerate() {
        match c {
            '[' => {
                if state == State::Text { flush_buffer!(buffer, result); }
                else { return Err(DisplayFormatError::Unexpected(pos, c)); }
                state = State::BeginBlock;
                context_pos = pos+1;
            }

            ':' => {
                if state == State::BeginBlock {
                    current_block = buffer.trim().to_string();
                    test_block_name!(context_pos, current_block);
                    context_pos = pos+1;
                    state = State::ArgumentList;
                    buffer.clear();
                } else {
                    buffer.push(c);
                }
            }

            ',' => {
                if state == State::ArgumentList {
                    if buffer.len() > 0 {
                        args.push(Some(parse_value(context_pos, buffer.trim())?));
                    } else {
                        args.push(None);
                    }
                    buffer.clear();
                    context_pos = pos+1;
                } else {
                    buffer.push(c);
                }
            }

            ']' => {
                // Blocks without arguments
                if state == State::BeginBlock {
                    let name = buffer.trim().to_string();
                    buffer.clear();
                    test_block_name!(context_pos, name);
                    validate_arguments(context_pos, &current_block, &args)?;

                    result.push(match name.as_str() {
                        "prev" => DisplayFormat::Prev,
                        "next" => DisplayFormat::Next,
                        "play-pause" => DisplayFormat::PlayPause,
                        "status" => DisplayFormat::Status,
                        "info" => DisplayFormat::PlayerInfo(
                            DEFAULT_INFO_SETTINGS.0, DEFAULT_INFO_SETTINGS.1,
                        ),

                        "metadata" => DisplayFormat::Metadata(
                            DEFAULT_META_SETTINGS.0, DEFAULT_META_SETTINGS.1,
                        ),

                        "time" => DisplayFormat::Time(
                            DEFAULT_TIME_SETTINGS.0, DEFAULT_TIME_SETTINGS.1,
                        ),

                        _ => unreachable!(),
                    });

                    state = State::Text;

                // Blocks with arguments
                } else if state == State::ArgumentList {
                    if buffer.len() > 0 {
                        args.push(Some(parse_value(context_pos, buffer.trim())?));
                        buffer.clear();
                    }
                    validate_arguments(context_pos, &current_block, &args)?;

                    result.push(match current_block.as_str() {
                        "info" => DisplayFormat::PlayerInfo(
                            extract_arg!(Bool, args, 0, DEFAULT_INFO_SETTINGS.0),
                            extract_arg!(Bool, args, 1, DEFAULT_INFO_SETTINGS.1),
                        ),

                        "metadata" => DisplayFormat::Metadata(
                            extract_arg!(Number, args, 0, DEFAULT_META_SETTINGS.0),
                            extract_arg!(Number, args, 1, DEFAULT_META_SETTINGS.1),
                        ),

                        "time" => DisplayFormat::Time(
                            extract_arg!(Bool, args, 0, DEFAULT_TIME_SETTINGS.0),
                            extract_arg!(Bool, args, 1, DEFAULT_TIME_SETTINGS.1),
                        ),

                        _ => unreachable!(),
                    });

                    args.clear();

                    state = State::Text;
                } else {
                    return Err(DisplayFormatError::Unexpected(pos, c));
                }
            }

            c => buffer.push(c),
        }
    }

    flush_buffer!(buffer, result);
    Ok(result)
}

pub fn process_meta_format(format: &str) -> Result<Vec<MetaFormat>, String> {
    let mut result = Vec::new();

    for chunk in format.split_terminator(|c| c == '[' || c == ']') {
        if chunk.len() > 0 {
            result.push(match chunk {
                "artist" => MetaFormat::Artist,
                "artists" => MetaFormat::Artists,
                "album" => MetaFormat::Album,
                "album-artist" => MetaFormat::AlbumArtist,
                "title" => MetaFormat::Title,
                "track" => MetaFormat::Track,
                s => MetaFormat::String(s.to_string()),
            });
        }
    }

    Ok(result)
}

#[test]
fn test_display_format() {
    use DisplayFormat::*;
    use super::DEFAULT_DISPLAY_FORMAT;

    assert_eq!(process_display_format(DEFAULT_DISPLAY_FORMAT).unwrap(), [
        Prev, String(" ".to_string()), PlayPause, String(" ".to_string()),
        Next, String(" ".to_string()), PlayerInfo(true, true),
        String(" ┃ ".to_string()), Metadata(32, 10),
    ]);

    assert_eq!(process_display_format("[[]").is_err(), true);
    assert_eq!(process_display_format("[prev]").unwrap(), [Prev]);
    assert_eq!(process_display_format("[metadata:]").unwrap(), [Metadata(32, 10)]);
    assert_eq!(process_display_format("[metadata:,]").unwrap(), [Metadata(32, 10)]);
    assert_eq!(process_display_format("[metadata:,11]").unwrap(), [Metadata(32, 11)]);
    assert_eq!(process_display_format("[metadata:,,]").is_err(), true);
}
