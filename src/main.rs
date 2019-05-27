use std::{env, thread, time::Duration, path::PathBuf, fs};
use std::io::{Read, Write, stdout};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use mpris::{DBusError, Player, PlayerFinder, PlaybackStatus, Metadata};
use formatting::*;

mod formatting;

const PLAY_ICON: &'static str = "";
const PAUSE_ICON: &'static str = "";
const STOPPED_ICON: &'static str = "";
const PREV_ICON: &'static str = "";
const NEXT_ICON: &'static str = "";
const CLOSED_MSG: &'static str = " no music playing";
const EMPTY_CHAR: char = '\u{ffff}';

const PIPE_PATH: &'static str = concat!("/tmp/cornetroll.", env!("USER"));
const DEFAULT_DISPLAY_FORMAT: &'static str = "[prev] [play-pause] [next] [info] ┃ [metadata]";
const DEFAULT_META_FORMAT: &'static str = "<[artist] - >[title]";
const DEFAULT_INFO_SETTINGS: (bool, bool) = (true, true);
const DEFAULT_META_SETTINGS: (u8, u8) = (32, 10);
const DEFAULT_TIME_SETTINGS: (bool, bool) = (true, false);
const COMMANDS: &[&'static str] = &[
    "play", "pause", "prev", "next",
    "prev-player", "next-player",
];

// If Strings and strs are guaranteed to hold a valid UTF-8 character, why the f*** does .len()
// return the size in bytes?
macro_rules! str_len {
    ($s:expr) => { $s.chars().count(); }
}

// Minimal Either enum
enum Either<L, R> {
    Left(L),
    Right(R),
}

struct Scroller {
    content: String,
    buffer: String,
    head: usize,
    forward: bool,
    wait: u8,
    size: usize,
    start_wait: u8,
}

struct Config {
    display_format: Vec<DisplayFormat>,
    meta_format: Vec<MetaFormat>,
    refresh_wait: u8,
}

struct PlayerStatus<'a> {
    bin_path: PathBuf,
    config: Config,
    finder: PlayerFinder,
    players: Vec<Player<'a>>,
    display_buffer: String,
    info_scroller: Scroller,
    meta_scroller: Scroller,
    current_idx: usize,
    refresh_wait: u8,
    last_display: String,
    _player_id: usize,
}

impl<'a> PlayerStatus<'a> {
    pub fn new(config: Config) -> Self {
        let mut me = Self {
            bin_path: env::current_exe().unwrap(),
            config,
            finder: PlayerFinder::new().unwrap(),
            players: Vec::new(),
            display_buffer: String::new(),
            info_scroller: Scroller::new(0, 0),
            meta_scroller: Scroller::new(0, 0),
            current_idx: 0,
            refresh_wait: 0,
            last_display: String::new(),
            _player_id: 0,
        };
        me.init_scrollers();
        me
    }

    pub fn refresh_cache(&mut self) {
        self.players = match self.finder.find_all() {
            Ok(vec) => vec,
            Err(_) => return,
        };
        if self.current_idx > self.players.len() { self.current_idx = 0; }
    }

    fn init_scrollers(&mut self) {
        for block in &self.config.display_format {
            match block {
                DisplayFormat::PlayerInfo(_, _) => {
                    self.info_scroller = Scroller::new(10, 6);
                },
                DisplayFormat::Metadata(buffer_size, scroller_wait) => {
                    self.meta_scroller = Scroller::new(*buffer_size, *scroller_wait);
                },
                _ => (),
            }
        }
    }

    pub fn update(&mut self) {
        if self.refresh_wait > 0 {
            self.refresh_wait -= 1;
        } else {
            self.refresh_cache();
            self.refresh_wait = self.config.refresh_wait;
        }

        if self.players.len() > 0 {
            if self.info_scroller.is_initialized() {
                self.info_scroller.set_content(&self.current_player().identity().to_string());
                self.info_scroller.update();
            }
            if let Ok(meta) = self.current_player().get_metadata() {
                if self.meta_scroller.is_initialized() {
                    self.update_meta(meta);
                }
            }
            //self.scroller.update();
        }
        self.display();
    }

    fn current_player(&self) -> &Player<'a> {
        &self.players[self.current_idx]
    }

    pub fn display(&mut self) {
        if self.players.len() > 0 {
            let status = match self.current_player().get_playback_status() {
                Ok(status) => status,
                Err(_) => {
                    // Disconnection
                    self.print_flush(self.last_display.clone());
                    self.refresh_cache();
                    return;
                },
            };

            self.display_buffer.clear();

            for block in self.config.display_format.iter() {
                let result = match block {
                    DisplayFormat::Prev => self.action("prev", PREV_ICON),
                    DisplayFormat::PlayPause => match status {
                        PlaybackStatus::Playing => self.action("pause", PAUSE_ICON),
                        _ => self.action("play", PLAY_ICON),
                    },
                    DisplayFormat::Next => self.action("next", NEXT_ICON),
                    DisplayFormat::Status => match status {
                        PlaybackStatus::Playing => PLAY_ICON.to_string(),
                        PlaybackStatus::Paused => PAUSE_ICON.to_string(),
                        PlaybackStatus::Stopped => STOPPED_ICON.to_string(),
                    },
                    DisplayFormat::PlayerInfo(show_total, show_name) => {
                        let mut info = String::new();
                        info.push_str(&format!("{}", self.current_idx+1));
                        if *show_total {
                            info.push_str(&format!("/{}", self.players.len()));
                        }
                        if *show_name {
                            info.push_str(": ");
                            info.push_str(self.info_scroller.display());
                        }
                        info
                    },
                    DisplayFormat::Metadata(_, _) => {
                        self.meta_scroller.display().to_string()
                    },
                    DisplayFormat::Time(show_length, use_remaining) => {
                        let mut time = String::new();

                        #[inline]
                        fn format_time(dur: Duration) -> String {
                            format!("{:02}:{:02}", dur.as_secs()/60, dur.as_secs() % 60)
                        }

                        let position = self.current_player().get_position();
                        let length = self.current_player().get_metadata().unwrap().length();
                        let remaining = if let Ok(p) = position {
                            if let Some(l) = length { Some(l-p) }
                            else { None }
                        } else {
                            None
                        };

                        if *show_length {
                            if let Ok(v) = position {
                                time.push_str(&format_time(v));
                            } else {
                                time.push_str(" N/A ");
                            }
                            time.push_str("/");

                            if *use_remaining {
                                if let Some(v) = remaining {
                                    time.push_str(&format_time(v));
                                } else {
                                    time.push_str(" N/A ");
                                }
                            } else {
                                if let Some(v) = length {
                                    time.push_str(&format_time(v));
                                } else {
                                    time.push_str(" N/A ");
                                }
                            }
                        } else {
                            if *use_remaining {
                                if let Some(v) = remaining {
                                    time.push_str(&format_time(v));
                                } else {
                                    time.push_str(" N/A ");
                                }
                            } else {
                                if let Ok(v) = position {
                                    time.push_str(&format_time(v));
                                } else {
                                    time.push_str(" N/A ");
                                }
                            }
                        }

                        time
                    },
                    DisplayFormat::String(s) => s.clone(),
                };
                self.display_buffer.push_str(&result);
            }

            self.print_flush(self.display_buffer.clone().trim_end());
        } else {
            self.print_flush(CLOSED_MSG)
        }
    }

    fn update_meta(&mut self, meta: Metadata) {
        const EMPTY_TAG: &str = "N/A";

        /// Tags are only Some if there's at least a non-empty string.
        macro_rules! validate_tag {
            ($tag:expr) => {
                match $tag {
                    Some(t) => if t.len() > 0 { Some(t) } else { None },
                    n => n,
                }
            };

            (list, $tag:expr) => {
                match $tag {
                    Some(list) => {
                        if list.len() > 0 {
                            if list[0].len() > 0 {
                                Some(list.iter().map(|e| e.as_str()).collect())
                            } else { None }
                        } else { None }
                    },
                    None => None,
                }
            };
        }

        struct Tags<'a> {
            artists: Option<Vec<&'a str>>,
            album_name: Option<&'a str>,
            album_artists: Option<Vec<&'a str>>,
            title: Option<&'a str>,
            track: Option<i32>,
        }

        let tags = &Tags {
            artists: validate_tag!(list, meta.artists()),
            album_name: validate_tag!(meta.album_name()),
            album_artists: validate_tag!(list, meta.album_artists()),
            title: validate_tag!(meta.title()),
            track: meta.track_number(),
        };

        let mut content = String::new();

        // Optionals render Strings before and after the first valid block
        fn build_content(content: &mut String, tags: &Tags, blocks: &Vec<MetaFormat>, optional: bool) {
            let mut flush_buffer = String::new();
            let mut flush = false;

            macro_rules! flushtag {
                (buffer) => {
                    if flush_buffer.len() > 0 && flush {
                        content.extend(flush_buffer.chars());
                        flush_buffer.clear();
                    }
                };

                (flush) => {
                    if !flush { flush = true; }
                };

                (unflush) => {
                    if flush {
                        flush = false;
                        if flush_buffer.len() > 0 { flush_buffer.clear(); }
                    }
                };

                ($tag:expr) => {
                    if optional {
                        if $tag.is_some() {
                            flushtag!(flush);
                            flushtag!(buffer);
                            content.push_str($tag.unwrap());
                        } else {
                            flushtag!(unflush);
                        }
                    } else {
                        content.push_str($tag.unwrap_or(EMPTY_TAG));
                    }
                };

                (number, $tag:expr) => {
                    if optional {
                        if $tag.is_some() {
                            flushtag!(flush);
                            flushtag!(buffer);
                            content.push_str(&format!("{}", $tag.unwrap()));
                        } else {
                            flushtag!(unflush);
                        }
                    } else {
                        if let Some(n) = $tag {
                            content.extend(format!("{}", n).chars());
                        }
                    }
                };

                (first, $tag:expr) => {
                    if optional {
                        if $tag.is_some() {
                            flushtag!(flush);
                            flushtag!(buffer);
                            content.push_str($tag.clone().unwrap()[0]);
                        } else {
                            flushtag!(unflush);
                        }
                    } else {
                        content.push_str(if let Some(list) = &$tag { list[0] } else { EMPTY_TAG });
                    }
                };

                (join, $tag:expr) => {
                    if optional {
                        if $tag.is_some() {
                            flushtag!(flush);
                            flushtag!(buffer);
                            content.extend($tag.clone().unwrap().join(", ").chars());
                        } else {
                            flushtag!(unflush);
                        }
                        if let Some(list) = &$tag {
                            content.extend(list.join(", ").chars());
                        } else {
                            content.push_str(EMPTY_TAG);
                        };
                    }
                };
            }

            for block in blocks {
                match block {
                    MetaFormat::Artist => flushtag!(first, tags.artists),
                    MetaFormat::Artists => flushtag!(join, tags.artists),
                    MetaFormat::Album => flushtag!(tags.album_name),
                    MetaFormat::AlbumArtist => flushtag!(first, tags.album_artists),
                    MetaFormat::Title => flushtag!(tags.title),
                    MetaFormat::Track => flushtag!(number, tags.track),
                    MetaFormat::String(s) => if optional { flush_buffer.push_str(&s); } else { content.push_str(&s); },
                    MetaFormat::Optional(o) => build_content(content, tags, &o, true),
                }
            }

            flushtag!(buffer);
        }

        build_content(&mut content, tags, &self.config.meta_format, false);
        let content = content.trim_end();
        self.meta_scroller.set_content(content);
        self.meta_scroller.update();
    }

    fn command(&mut self, command: &str) -> Result<(), DBusError> {
        if self.players.len() == 0 { return Ok(()); }

        match command {
            "play" => self.current_player().play()?,
            "pause" => self.current_player().pause()?,
            "stop" => self.current_player().stop()?,
            "prev" => self.current_player().previous()?,
            "next" => self.current_player().next()?,
            "next-player" => {
                if self.current_idx < self.players.len()-1 {
                    self.current_idx += 1;
                }
            },
            "prev-player" => {
                if self.current_idx > 0 {
                    self.current_idx -= 1;
                }
            },
            _ => (),
        }

        Ok(())
    }

    fn action(&self, command: &str, icon: &str) -> String {
        format!("%{{A1:{} {}:}}{}%{{A}}", self.bin_path.display(), command, icon)
    }

    fn print_flush<S: AsRef<str>>(&mut self, string: S) {
        let string = string.as_ref();
        if string != self.last_display {
            // Use oneliner for debugging
            #[cfg(debug_assertions)]
            print!("\r{}\r{}", " ".repeat(self.last_display.len()), string);
            #[cfg(not(debug_assertions))]
            println!("{}", string);

            stdout().flush().unwrap();
            self.last_display = string.to_string();
        }
    }
}

impl Scroller {
    pub fn new(size: u8, wait: u8) -> Self {
        Scroller {
            content: String::new(),
            buffer: String::new(),
            head: 0,
            forward: true,
            wait,
            size: size as usize,
            start_wait: wait,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.size > 0
    }

    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
    }

    pub fn update(&mut self) {
        use std::cmp::min;

        let content_len = str_len!(self.content);

        if content_len > self.size {
            if self.wait > 0 { self.wait -= 1; }
            if self.forward && self.head < content_len-self.size && self.wait == 0 {
                self.head += 1;
            } else if self.forward && self.head == content_len-self.size {
                self.forward = false;
                self.wait = self.start_wait;
            } else if !self.forward && self.head > 0 && self.wait == 0 {
                self.head -= 1;
            } else if !self.forward && self.head == 0 {
                self.forward = true;
                self.wait = self.start_wait;
            }
        } else {
            if self.head > 0 { self.head = 0; }
            self.wait = self.start_wait;
        }

        let chars = self.content.chars().skip(self.head);
        let size = min(self.size, content_len-self.head);
        self.buffer.clear();
        self.buffer.push_str(&chars.take(size).collect::<String>());

        let buffer_len = str_len!(self.buffer);
        if buffer_len < self.size {
            self.buffer.extend(" ".repeat(self.size-buffer_len).chars());
        }

        // Polybar strips the module's output, so scrollers at the end
        // will not work properly.
        self.buffer.push(EMPTY_CHAR);
    }

    pub fn display(&self) -> &str {
        &self.buffer
    }
}

fn parse_cli() -> Result<Either<String, Config>, String> {
    use clap::{App, Arg};

    let matches = App::new("cornetroll")
        .version(env!("CARGO_PKG_VERSION"))
        .author("manokara <marknokalt@live.com>")
        .about("MPRIS2 controller applet for polybar")
        .arg(Arg::with_name("command")
             .help("Which command to send to the current running instance")
             .possible_values(COMMANDS)
        )
        .arg(Arg::with_name("display-format")
             .help("How the player presents itself")
             .short("f")
             .long("display-format")
             .takes_value(true)
             .default_value(DEFAULT_DISPLAY_FORMAT)
        )
        .arg(Arg::with_name("metadata-format")
             .help("What information about the song will be shown")
             .short("m")
             .long("metadata-format")
             .takes_value(true)
             .default_value(DEFAULT_META_FORMAT)
        )
        .arg(Arg::with_name("refresh-ticks")
             .help("How many ticks to wait to refresh the player cache.")
             .short("r")
             .long("refresh-ticks")
             .takes_value(true)
             .default_value("10")
        )
    .get_matches();

    if let Some(command) = matches.value_of("command") {
        Ok(Either::Left(command.to_string()))
    } else {
        let display_format = match process_display_format(matches.value_of("display-format").unwrap()) {
            Ok(v) => v,
            Err(e) => return Err(format!("Display format - {}", e)),
        };
        let meta_format = match process_meta_format(matches.value_of("metadata-format").unwrap()) {
            Ok(v) => v,
            Err(e) => return Err(format!("Metadata format - {}", e)),
        };

        let mut metadata_test = false;
        for fmt in &display_format {
            if let DisplayFormat::Metadata(_, _) = fmt {
                metadata_test = true;
                break;
            }
        }

        if !metadata_test {
            return Err("Display format has no metadata block.".to_string());
        }

        Ok(Either::Right(Config {
            display_format,
            meta_format,
            refresh_wait: matches.value_of("refresh-ticks").unwrap().parse::<u8>()
                        .map_err(|_| "refresh-ticks must be between 0 and 255 inclusive.")?,
        }))
    }
}

fn main() {
    match parse_cli() {
        Ok(e) => match e {
            Either::Left(command) => {
                let mut pipe = unix_named_pipe::open_write(PIPE_PATH).expect("Unable to write to named ppipe");
                pipe.write_all(command.as_bytes()).unwrap();
            }

            Either::Right(config) => {
                let term = Arc::new(AtomicBool::new(false));
                #[cfg(debug_assertions)]
                signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).expect("Signal mayhem!");
                #[cfg(not(debug_assertions))]
                signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).expect("Signal mayhem!");

                let mut status = PlayerStatus::new(config);

                // Setup pipe
                match fs::remove_file(PIPE_PATH) {
                    Ok(_) => (),
                    Err(_) => (),
                }
                unix_named_pipe::create(PIPE_PATH, Some(0o600)).expect("Couldn't create named pipe");
                let mut pipe = unix_named_pipe::open_read(PIPE_PATH).expect("Unable to open named pipe");
                let mut pipe_buffer = String::new();

                while !term.load(Ordering::Relaxed) {
                    pipe.read_to_string(&mut pipe_buffer).expect("Unable to read named pipe");
                    if pipe_buffer.len() > 0 {
                        if let Ok(_) = status.command(&pipe_buffer) {};
                    }
                    pipe_buffer.clear();
                    status.update();
                    thread::sleep(Duration::from_millis(300));
                }

                fs::remove_file(PIPE_PATH).unwrap();
            }
        }

        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    }
}
