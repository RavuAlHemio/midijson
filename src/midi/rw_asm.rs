use std::fmt;
use std::io::{self, BufRead, Write};

use crate::midi::model::{
    ChannelValueMessage, Control, ControlChangeMessage, Event, EventData, FileHeader, KnownMaxValue,
    Message, MetaEventData, MetaType, MidiEventData, NoteMessage, PitchBendMessage,
    ProgramChangeMessage, SmfFormat, SongPositionData, SongSelectData, StandardMidiFile,
    SysExEventData, SystemEventData, TimeCodeData, Track, U14, U3, U4, U7,
};


#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    TrackCountMismatch { header: u16, file: usize },
    ExpectedHeader {
        line_number: usize,
        expected_mnemonic: &'static str,
        obtained_mnemonic: String,
    },
    WrongArgumentCount {
        line_number: usize,
        mnemonic: &'static str,
        expected: usize,
        obtained: usize,
    },
    InvalidInteger { line_number: usize, integer_string: String },
    IntegerTooLarge { line_number: usize, value: u64, max_value: u64 },
    IntegerTooSmall { line_number: usize, value: i64, min_value: i64 },
    UnknownMnemonic { line_number: usize, mnemonic: String },
}
impl Error {
    pub fn ensure_args(line_number: usize, mnemonic: &'static str, expected: usize, chunks_len: usize) -> Result<(), Self> {
        assert!(chunks_len > 0);
        if chunks_len - 1 == expected {
            Ok(())
        } else {
            Err(Self::WrongArgumentCount {
                line_number,
                mnemonic,
                expected,
                obtained: chunks_len - 1,
            })
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::TrackCountMismatch { header, file }
                => write!(f, "track number mismatch between header ({}) and file ({})", header, file),
            Self::ExpectedHeader { line_number, expected_mnemonic, obtained_mnemonic }
                => write!(f, "line {}: expected header mnemonic {:?}, obtained {:?}", line_number, expected_mnemonic, obtained_mnemonic),
            Self::WrongArgumentCount { line_number, mnemonic, expected, obtained }
                => write!(f, "line {}: mnemonic {:?} expects {} arguments, obtained {}", line_number, mnemonic, expected, obtained),
            Self::InvalidInteger { line_number, integer_string: number_string }
                => write!(f, "line {}: invalid integer {:?}", line_number, number_string),
            Self::IntegerTooLarge { line_number, value, max_value }
                => write! (f, "line {}: integer {} too large (maximum {} allowed)", line_number, value, max_value),
            Self::IntegerTooSmall { line_number, value, min_value }
                => write! (f, "line {}: integer {} too small (minimum {} allowed)", line_number, value, min_value),
            Self::UnknownMnemonic { line_number, mnemonic }
                => write!(f, "line {}: unknown mnemonic {:?}", line_number, mnemonic),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::TrackCountMismatch { .. } => None,
            Self::ExpectedHeader { .. } => None,
            Self::WrongArgumentCount { .. } => None,
            Self::InvalidInteger { .. } => None,
            Self::IntegerTooLarge { .. } => None,
            Self::IntegerTooSmall { .. } => None,
            Self::UnknownMnemonic { .. } => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}


pub fn write_asm<W: Write>(smf: &StandardMidiFile, writer: &mut W) -> Result<(), Error> {
    if usize::from(smf.header.track_count) != smf.tracks.len() {
        return Err(Error::TrackCountMismatch { header: smf.header.track_count, file: smf.tracks.len() });
    }

    writeln!(writer, "fmt {}", smf.header.format.to_base_type())?;
    writeln!(writer, "div {}", smf.header.division)?;

    for track in &smf.tracks {
        write_asm_track(track, writer)?;
    }

    Ok(())
}

fn write_asm_track<W: Write>(track: &Track, writer: &mut W) -> Result<(), Error> {
    writeln!(writer, "trk")?;

    for event in &track.events {
        if event.delta_time > 0 {
            write!(writer, "{} ", event.delta_time)?;
        }
        match &event.data {
            EventData::Midi(data) => {
                write!(writer, "ch {} ", data.channel.to_base_type())?;
                match &data.message {
                    Message::NoteOff(msg) => {
                        writeln!(writer, "nof {} {}", msg.note.to_base_type(), msg.value.to_base_type())?;
                    },
                    Message::NoteOn(msg) => {
                        writeln!(writer, "non {} {}", msg.note.to_base_type(), msg.value.to_base_type())?;
                    },
                    Message::KeyPressure(msg) => {
                        writeln!(writer, "kpr {} {}", msg.note.to_base_type(), msg.value.to_base_type())?;
                    },
                    Message::ControlChange(msg) => {
                        writeln!(writer, "cch {} {}", msg.control.to_base_type().to_base_type(), msg.value.to_base_type())?;
                    },
                    Message::ProgramChange(msg) => {
                        writeln!(writer, "pch {}", msg.program.to_base_type())?;
                    },
                    Message::ChannelPressure(msg) => {
                        writeln!(writer, "cpr {}", msg.value.to_base_type())?;
                    },
                    Message::PitchBend(msg) => {
                        writeln!(writer, "pbd {}", msg.value.to_base_type())?;
                    },
                }
            },
            EventData::System(system_data) => {
                match system_data {
                    SystemEventData::TimeCode(data) => {
                        writeln!(writer, "tcd {} {}", data.message_type.to_base_type(), data.data.to_base_type())?;
                    },
                    SystemEventData::SongPosition(data) => {
                        writeln!(writer, "sps {}", data.position.to_base_type())?;
                    },
                    SystemEventData::SongSelect(data) => {
                        writeln!(writer, "ssl {}", data.song.to_base_type())?;
                    },
                    SystemEventData::TuneRequest => {
                        writeln!(writer, "trq")?;
                    },
                    SystemEventData::TimingClock => {
                        writeln!(writer, "tcl")?;
                    },
                    SystemEventData::F9 => {
                        writeln!(writer, "uf9")?;
                    },
                    SystemEventData::Start => {
                        writeln!(writer, "srt")?;
                    },
                    SystemEventData::Continue => {
                        writeln!(writer, "cnt")?;
                    },
                    SystemEventData::Stop => {
                        writeln!(writer, "stp")?;
                    },
                    SystemEventData::FD => {
                        writeln!(writer, "ufd")?;
                    },
                    SystemEventData::ActiveSensing => {
                        writeln!(writer, "asg")?;
                    },
                    SystemEventData::SystemReset => {
                        writeln!(writer, "rst")?;
                    },
                }
            },
            EventData::SysEx(data) => {
                write!(writer, "syx")?;
                for b in &data.data {
                    write!(writer, " {:#04X}", *b)?;
                }
                writeln!(writer)?;
            },
            EventData::RawSysEx(data) => {
                writeln!(writer, "rsx")?;
                for b in &data.data {
                    write!(writer, " {:#04X}", *b)?;
                }
                writeln!(writer)?;
            },
            EventData::Meta(data) => {
                write!(writer, "mta {}", data.meta_type.to_base_type().to_base_type())?;
                for b in &data.data {
                    write!(writer, " {:#04X}", *b)?;
                }
                writeln!(writer)?;
            },
        }
    }

    Ok(())
}

fn fold_whitespace(s: &mut String) {
    // leading whitespace?
    match s.find(|c: char| !c.is_whitespace()) {
        Some(0) => {
            // no leading whitespace; keep going
        },
        Some(n) => {
            // strip that first
            s.replace_range(0..n, "");
        },
        None => {
            // only whitespace, heh
            s.clear();
            return;
        },
    }

    let mut start_index = 0;

    while let Some(relative_ws_index) = s[start_index..].find(|c: char| c.is_whitespace()) {
        // okay, we found whitespace
        let ws_index= start_index + relative_ws_index;

        // how many bytes is the whitespace character?
        let ws_char_len = s[ws_index..].chars().nth(0).unwrap().len_utf8();

        // find the first character that is not whitespace
        match s[ws_index+ws_char_len..].find(|c: char| !c.is_whitespace()) {
            Some(relative_non_ws_index) => {
                // found something that is not whitespace
                let non_ws_index = ws_index + ws_char_len + relative_non_ws_index;

                // replace all of that whitespace with a single space
                if &s[ws_index..non_ws_index] != " " {
                    s.replace_range(ws_index..non_ws_index, " ");
                }

                // continue right after ws_index + the space
                start_index = ws_index + ' '.len_utf8();
            },
            None => {
                // the rest of the string is whitespace

                // trim it to only go as far as ws_index and stop
                s.truncate(ws_index);
                break;
            },
        }
    }
}

macro_rules! number_parser {
    ($name:ident, $type:ty) => {
        fn $name(s: &str) -> Option<$type> {
            let no_underscores = s.replace("_", "");
            if let Some(hex_str) = no_underscores.strip_prefix("0x") {
                <$type>::from_str_radix(hex_str, 16).ok()
            } else if let Some(bin_str) = no_underscores.strip_prefix("0b") {
                <$type>::from_str_radix(bin_str, 2).ok()
            } else {
                // no octal, octal is pointless on machines with multiples of 8 bits
                <$type>::from_str_radix(&no_underscores, 10).ok()
            }
        }
    };
}

number_parser!(parse_unsigned, u64);
number_parser!(parse_signed, i64);

macro_rules! unsigned_thinning_parser {
    ($name:ident, $type:ty, $base_func:ident) => {
        fn $name(s: &str, line_number: usize) -> Result<$type, Error> {
            let num = $base_func(s)
                .ok_or_else(|| Error::InvalidInteger { line_number, integer_string: s.to_owned() })?;
            if num > <$type>::known_max_value().into() {
                Err(Error::IntegerTooLarge { line_number, value: num, max_value: <$type>::known_max_value().into() })
            } else {
                Ok(num.try_into().unwrap())
            }
        }
    };
}
macro_rules! signed_thinning_parser {
    ($name:ident, $type:ty, $base_func:ident) => {
        fn $name(s: &str, line_number: usize) -> Result<$type, Error> {
            let num = $base_func(s)
                .ok_or_else(|| Error::InvalidInteger { line_number, integer_string: s.to_owned() })?;
            if num > <$type>::MAX.into() {
                Err(Error::IntegerTooLarge { line_number, value: num as u64, max_value: <$type>::MAX as u64 })
            } else if num < <$type>::MIN.into() {
                Err(Error::IntegerTooSmall { line_number, value: num, min_value: <$type>::MIN.into() })
            } else {
                Ok(num as $type)
            }
        }
    };
}

unsigned_thinning_parser!(parse_u3, U3, parse_unsigned);
unsigned_thinning_parser!(parse_u4, U4, parse_unsigned);
unsigned_thinning_parser!(parse_u7, U7, parse_unsigned);
unsigned_thinning_parser!(parse_u8, u8, parse_unsigned);
unsigned_thinning_parser!(parse_u14, U14, parse_unsigned);
unsigned_thinning_parser!(parse_u16, u16, parse_unsigned);
unsigned_thinning_parser!(parse_u32, u32, parse_unsigned);
signed_thinning_parser!(parse_i16, i16, parse_signed);

pub fn read_asm<R: BufRead>(reader: &mut R) -> Result<StandardMidiFile, Error> {
    let mut format = None;
    let mut division = None;
    let mut track_started = false;
    let mut current_track = Vec::new();
    let mut tracks = Vec::new();

    let mut line = String::new();
    let mut line_number = 0;
    loop {
        line.clear();

        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            // EOF
            break;
        }
        line_number += 1;

        // strip off a comment
        if let Some(hash_index) = line.find('#') {
            line.truncate(hash_index);
        }

        // fold whitespace (strip leading + trailing, compress all others into a single U+0020)
        fold_whitespace(&mut line);
        if line.len() == 0 {
            continue;
        }
        let chunks: Vec<&str> = line.split(' ').collect();

        // make sure we have the header
        if format.is_none() {
            if chunks[0] == "fmt" {
                Error::ensure_args(line_number, "fmt", 1, chunks.len())?;

                let format_number = parse_u16(&chunks[1], line_number)?;
                format = Some(SmfFormat::from_base_type(format_number));
                continue;
            } else {
                return Err(Error::ExpectedHeader {
                    line_number,
                    expected_mnemonic: "fmt",
                    obtained_mnemonic: chunks[0].to_owned(),
                });
            }
        }

        if division.is_none() {
            if chunks[0] == "div" {
                Error::ensure_args(line_number, "div ", 1, chunks.len())?;

                let division_number = parse_i16(&chunks[1], line_number)?;
                division = Some(division_number);
                continue;
            } else {
                return Err(Error::ExpectedHeader {
                    line_number,
                    expected_mnemonic: "div",
                    obtained_mnemonic: chunks[0].to_owned(),
                });
            }
        }

        if !track_started {
            if chunks[0] == "trk" {
                Error::ensure_args(line_number, "trk", 0, chunks.len())?;
                track_started = true;
                continue;
            } else {
                return Err(Error::ExpectedHeader {
                    line_number,
                    expected_mnemonic: "trk",
                    obtained_mnemonic: chunks[0].to_owned(),
                });
            }
        }

        if chunks[0] == "trk" {
            // new track
            Error::ensure_args(line_number, "trk", 0, chunks.len())?;
            let finished_track_events = std::mem::replace(&mut current_track, Vec::new());
            tracks.push(Track {
                events: finished_track_events,
            });
            continue;
        }

        // the upcoming events may be prefixed with a delta time

        let (delta_time, mnemo_chunks) = if parse_unsigned(chunks[0]).is_some() {
            let dt = parse_u32(chunks[0], line_number)?;
            (dt, &chunks[1..])
        } else {
            (0, chunks.as_slice())
        };

        match mnemo_chunks[0] {
            "ch" => {
                // some channel event

                // we need at least three chunks
                if mnemo_chunks.len() < 3 {
                    return Err(Error::WrongArgumentCount {
                        line_number,
                        mnemonic: "ch",
                        expected: 2,
                        obtained: mnemo_chunks.len() - 1,
                    });
                }

                // which channel?
                let channel = parse_u4(mnemo_chunks[1], line_number)?;

                // what event?
                let message = match mnemo_chunks[2] {
                    "nof" => {
                        // note off
                        Error::ensure_args(line_number, "ch.nof", 4, mnemo_chunks.len())?;
                        let note = parse_u7(mnemo_chunks[3], line_number)?;
                        let value = parse_u7(mnemo_chunks[4], line_number)?;
                        Message::NoteOff(NoteMessage {
                            note,
                            value,
                        })
                    },
                    "non" => {
                        // note on
                        Error::ensure_args(line_number, "ch.non", 4, mnemo_chunks.len())?;
                        let note = parse_u7(mnemo_chunks[3], line_number)?;
                        let value = parse_u7(mnemo_chunks[4], line_number)?;
                        Message::NoteOn(NoteMessage {
                            note,
                            value,
                        })
                    },
                    "kpr" => {
                        // key pressure
                        Error::ensure_args(line_number, "ch.kpr", 4, mnemo_chunks.len())?;
                        let note = parse_u7(mnemo_chunks[3], line_number)?;
                        let value = parse_u7(mnemo_chunks[4], line_number)?;
                        Message::KeyPressure(NoteMessage {
                            note,
                            value,
                        })
                    },
                    "cch" => {
                        // control change
                        Error::ensure_args(line_number, "ch.cch", 4, mnemo_chunks.len())?;
                        let control = parse_u7(mnemo_chunks[3], line_number)?;
                        let value = parse_u7(mnemo_chunks[4], line_number)?;
                        Message::ControlChange(ControlChangeMessage {
                            control: Control::from_base_type(control),
                            value,
                        })
                    },
                    "pch" => {
                        // program change
                        Error::ensure_args(line_number, "ch.pch", 3, mnemo_chunks.len())?;
                        let program = parse_u7(mnemo_chunks[3], line_number)?;
                        Message::ProgramChange(ProgramChangeMessage {
                            program,
                        })
                    },
                    "cpr" => {
                        // channel pressure
                        Error::ensure_args(line_number, "ch.cpr", 4, mnemo_chunks.len())?;
                        let value = parse_u7(mnemo_chunks[3], line_number)?;
                        Message::ChannelPressure(ChannelValueMessage {
                            value,
                        })
                    },
                    "pbd" => {
                        // pitch bend
                        Error::ensure_args(line_number, "ch.pbd", 3, mnemo_chunks.len())?;
                        let value = parse_u14(mnemo_chunks[3], line_number)?;
                        Message::PitchBend(PitchBendMessage {
                            value,
                        })
                    },
                    other => return Err(Error::UnknownMnemonic { line_number, mnemonic: other.to_owned() }),
                };
                current_track.push(Event {
                    delta_time,
                    data: EventData::Midi(MidiEventData {
                        channel,
                        message,
                    }),
                });
            },

            // system events
            "tcd" => {
                // time code
                Error::ensure_args(line_number, "tcd", 2, mnemo_chunks.len())?;
                let message_type = parse_u3(mnemo_chunks[1], line_number)?;
                let data = parse_u4(mnemo_chunks[2], line_number)?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::TimeCode(TimeCodeData {
                        message_type,
                        data,
                    })),
                });
            },
            "sps" => {
                // song position
                Error::ensure_args(line_number, "sps", 1, mnemo_chunks.len())?;
                let position = parse_u14(mnemo_chunks[1], line_number)?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::SongPosition(SongPositionData {
                        position,
                    })),
                });
            },
            "ssl" => {
                // song select
                Error::ensure_args(line_number, "ssl", 2, mnemo_chunks.len())?;
                let song = parse_u7(mnemo_chunks[1], line_number)?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::SongSelect(SongSelectData {
                        song,
                    })),
                });
            },
            "trq" => {
                // tune request
                Error::ensure_args(line_number, "trq", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::TuneRequest),
                });
            },
            "tcl" => {
                // timing clock
                Error::ensure_args(line_number, "tcl", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::TimingClock),
                });
            },
            "uf9" => {
                // unknown 0xF9
                Error::ensure_args(line_number, "uf9", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::F9),
                });
            },
            "srt" => {
                // start
                Error::ensure_args(line_number, "srt", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::Start),
                });
            },
            "cnt" => {
                // continue
                Error::ensure_args(line_number, "cnt", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::Continue),
                });
            },
            "stp" => {
                // stop
                Error::ensure_args(line_number, "stp", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::Stop),
                });
            },
            "ufd" => {
                // unknown 0xFD
                Error::ensure_args(line_number, "ufd", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::FD),
                });
            },
            "asg" => {
                // active sensing
                Error::ensure_args(line_number, "asg", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::ActiveSensing),
                });
            },
            "rst" => {
                // system reset
                Error::ensure_args(line_number, "rst", 0, mnemo_chunks.len())?;
                current_track.push(Event {
                    delta_time,
                    data: EventData::System(SystemEventData::SystemReset),
                });
            },
            // end of system events

            "syx"|"rsx" => {
                // SysEx and raw SysEx
                let mut data = Vec::with_capacity(mnemo_chunks.len() - 1);
                for chunk in &mnemo_chunks[1..] {
                    let byte = parse_u8(chunk, line_number)?;
                    data.push(byte);
                }
                let event_data = if mnemo_chunks[0] == "syx" {
                    EventData::SysEx(SysExEventData { data })
                } else {
                    EventData::RawSysEx(SysExEventData { data })
                };
                current_track.push(Event {
                    delta_time,
                    data: event_data,
                });
            },
            "mta" => {
                // metadata
                if mnemo_chunks.len() < 2 {
                    return Err(Error::WrongArgumentCount {
                        line_number,
                        mnemonic: "mta",
                        expected: 2,
                        obtained: mnemo_chunks.len() - 1,
                    });
                }
                let meta_type_number = parse_u7(mnemo_chunks[1], line_number)?;
                let meta_type = MetaType::from_base_type(meta_type_number);
                let mut data = Vec::with_capacity(mnemo_chunks.len() - 2);
                for chunk in &mnemo_chunks[2..] {
                    let byte = parse_u8(chunk, line_number)?;
                    data.push(byte);
                }
                current_track.push(Event {
                    delta_time,
                    data: EventData::Meta(MetaEventData {
                        meta_type,
                        data,
                    }),
                });
            },
            other => return Err(Error::UnknownMnemonic { line_number, mnemonic: other.to_owned() }),
        }
    }

    tracks.push(Track {
        events: current_track,
    });

    let concrete_format = match format {
        Some(f) => f,
        None => return Err(Error::ExpectedHeader {
            line_number,
            expected_mnemonic: "fmt",
            obtained_mnemonic: String::new(),
        }),
    };
    let concrete_division = match division {
        Some(d) => d,
        None => return Err(Error::ExpectedHeader {
            line_number,
            expected_mnemonic: "div",
            obtained_mnemonic: String::new(),
        }),
    };

    Ok(StandardMidiFile {
        header: FileHeader {
            format: concrete_format,
            track_count: tracks.len().try_into().unwrap(),
            division: concrete_division,
        },
        tracks,
    })
}
