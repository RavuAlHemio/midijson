use std::fmt;
use std::io::{self, BufRead, Write};

use crate::midi::model::{EventData, Message, StandardMidiFile, SystemEventData, Track};


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
                writeln!(writer, "mta {}", data.meta_type.to_base_type().to_base_type())?;
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

pub fn read_asm<R: BufRead>(reader: &mut R) -> Result<StandardMidiFile, Error> {
    let mut format = None;
    let mut divisions = None;

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
        let mnemonic = chunks[0];

        // make sure we have the header
        if format.is_none() {
            if mnemonic == "fmt" {
                if chunks.len() != 2 {
                    return Err(Error::WrongArgumentCount {
                        line_number,
                        mnemonic: "fmt",
                        expected: 1,
                        obtained: chunks.len() - 1,
                    });
                }

                crate::midi::model::SmfFormat::from_base_type(base_value)

                let format_number: u16 = chunks[1].parse();
            } else {
                return Err(Error::ExpectedHeader {
                    line_number,
                    expected_mnemonic: "fmt",
                    obtained_mnemonic: mnemonic.to_owned(),
                });
            }
        }
    }

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
                writeln!(writer, "mta {}", data.meta_type.to_base_type().to_base_type())?;
                for b in &data.data {
                    write!(writer, " {:#04X}", *b)?;
                }
                writeln!(writer)?;
            },
        }
    }

    Ok(())
}
