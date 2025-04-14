//! Reading and writing Standard MIDI Files.


use std::fmt;
use std::io::{self, Read, Write};

use crate::midi::model::{
    ChannelValueMessage, ControlChangeMessage, Event, EventData, FileHeader, Message, MetaEventData,
    MidiEventData, NoteMessage, PitchBendMessage, ProgramChangeMessage, SmfFormat, SongPositionData,
    SongSelectData, SysExEventData, SystemEventData, StandardMidiFile, TimeCodeData, Track, U14, U3,
    U4, U7,
};


macro_rules! ensure_func {
    ($name:ident, $data_type:ty, $variant:ident) => {
        #[must_use]
        pub fn $name(expected: $data_type, obtained: $data_type) -> Result<(), Self> {
            if expected == obtained {
                Ok(())
            } else {
                Err(Self::$variant { expected, obtained })
            }
        }
    }
}


#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    UnexpectedHeaderChunk { expected: [u8; 4], obtained: [u8; 4] },
    UnexpectedHeaderLength { expected: u32, obtained: u32 },
    UnexpectedTrackChunk { expected: [u8; 4], obtained: [u8; 4] },
    VariableLengthQuantityTooLarge { maximum: u64, obtained: u64 },
    UnterminatedVariableLengthQuantity,
    EmptyEvent,
    UnknownPreviousEvent,
    NotEnoughParameters,
    ParameterHighBitSet,
    MoreLengthThanData,
    UnknownMessage { status_byte: u8 },
    TrackCountMismatch { header: u16, file: usize }
}
impl Error {
    ensure_func!(ensure_header_chunk, [u8; 4], UnexpectedHeaderChunk);
    ensure_func!(ensure_header_length, u32, UnexpectedHeaderLength);
    ensure_func!(ensure_track_chunk, [u8; 4], UnexpectedTrackChunk);
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e)
                => write!(f, "I/O error: {}", e),
            Self::UnexpectedHeaderChunk { expected, obtained }
                => write!(f, "expected header chunk {:?}, obtained {:?}", expected, obtained),
            Self::UnexpectedHeaderLength { expected, obtained }
                => write!(f, "expected header length {:?}, obtained {:?}", expected, obtained),
            Self::UnexpectedTrackChunk { expected, obtained }
                => write!(f, "expected track chunk {:?}, obtained {:?}", expected, obtained),
            Self::VariableLengthQuantityTooLarge { maximum, obtained }
                => write!(f, "variable-length quantity 0x{:X} too large; maximum allowed is 0x{:X}", obtained, maximum),
            Self::UnterminatedVariableLengthQuantity
                => write!(f, "variable-length quantity not terminated correctly"),
            Self::EmptyEvent
                => write!(f, "MIDI event with no values"),
            Self::UnknownPreviousEvent
                => write!(f, "MIDI event without a status byte and no previous status byte known"),
            Self::NotEnoughParameters
                => write!(f, "event with too few parameters"),
            Self::ParameterHighBitSet
                => write!(f, "a required parameter has the high bit set (declares itself a status byte)"),
            Self::MoreLengthThanData
                => write!(f, "length value is greater than the amount of remaining data"),
            Self::UnknownMessage { status_byte }
                => write!(f, "unknown message with status byte 0x{:02X}", status_byte),
            Self::TrackCountMismatch { header, file }
                => write!(f, "track number mismatch between header ({}) and file ({})", header, file),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::UnexpectedHeaderChunk { .. } => None,
            Self::UnexpectedHeaderLength { .. } => None,
            Self::UnexpectedTrackChunk { .. } => None,
            Self::VariableLengthQuantityTooLarge { .. } => None,
            Self::UnterminatedVariableLengthQuantity => None,
            Self::EmptyEvent => None,
            Self::UnknownPreviousEvent => None,
            Self::NotEnoughParameters => None,
            Self::ParameterHighBitSet => None,
            Self::MoreLengthThanData => None,
            Self::UnknownMessage { .. } => None,
            Self::TrackCountMismatch { .. } => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}


pub fn read_smf<R: Read>(reader: &mut R) -> Result<StandardMidiFile, Error> {
    let mut two_buf = [0u8; 2];
    let mut four_buf = [0u8; 4];

    let header = {
        // expect the header chunk
        reader.read_exact(&mut four_buf)?;
        Error::ensure_header_chunk(*b"MThd", four_buf)?;

        // expect a length
        reader.read_exact(&mut four_buf)?;
        let header_length = u32::from_be_bytes(four_buf);
        Error::ensure_header_length(6, header_length)?;

        // read the header
        reader.read_exact(&mut two_buf)?;
        let smf_format_u16 = u16::from_be_bytes(two_buf);
        let smf_format = SmfFormat::from_base_type(smf_format_u16);
        reader.read_exact(&mut two_buf)?;
        let track_count = u16::from_be_bytes(two_buf);
        reader.read_exact(&mut two_buf)?;
        let division = i16::from_be_bytes(two_buf);

        FileHeader {
            format: smf_format,
            track_count,
            division,
        }
    };

    let mut tracks = Vec::with_capacity(header.track_count.into());
    for _ in 0..header.track_count {
        // expect a track chunk
        reader.read_exact(&mut four_buf)?;
        Error::ensure_track_chunk(*b"MTrk", four_buf)?;

        reader.read_exact(&mut four_buf)?;
        let track_length = u32::from_be_bytes(four_buf);

        let track_length_usize = track_length.try_into().unwrap();
        let mut track_data = vec![0u8; track_length_usize];
        reader.read_exact(&mut track_data)?;

        let mut events = Vec::new();
        let mut track_slice = track_data.as_slice();
        let mut last_event_byte = None;
        while track_slice.len() > 0 {
            let (slice_rest, event) = take_smf_event(track_slice, &mut last_event_byte)?;
            events.push(event);
            track_slice = slice_rest;
        }

        tracks.push(Track {
            events,
        });
    }

    Ok(StandardMidiFile {
        header,
        tracks,
    })
}

fn take_smf_event<'e, 'l>(slice: &'e [u8], last_event_byte: &'l mut Option<u8>) -> Result<(&'e [u8], Event), Error> {
    let (slice, delta_time) = take_variable_length_quantity(slice)?;
    if slice.len() == 0 {
        return Err(Error::EmptyEvent);
    }
    let (slice, status_byte) = if slice[0] & 0b1000_0000 != 0 {
        (&slice[1..], slice[0])
    } else if let Some(leb) = *last_event_byte {
        (slice, leb)
    } else {
        return Err(Error::UnknownPreviousEvent);
    };

    // remember for next time
    *last_event_byte = Some(status_byte);

    let (slice, event_data) = match status_byte {
        0x00..=0x7F => unreachable!(),
        0x80..=0xEF => {
            // regular MIDI channel message
            let channel = U4::try_from(status_byte & 0b0000_1111).unwrap();
            let (slice, message) = match status_byte & 0xF0 {
                0x80|0x90|0xA0 => {
                    // note off, note on, key pressure
                    // two data bytes
                    let (slice, parameters): (_, [U7; 2]) = take_parameters(slice)?;
                    let note_message = NoteMessage {
                        note: parameters[0],
                        value: parameters[1],
                    };
                    let message = match status_byte & 0xF0 {
                        0x80 => Message::NoteOff(note_message),
                        0x90 => Message::NoteOn(note_message),
                        0xA0 => Message::KeyPressure(note_message),
                        _ => unreachable!(),
                    };
                    (slice, message)
                },
                0xB0 => {
                    // control change
                    // two data bytes
                    let (slice, parameters): (_, [U7; 2]) = take_parameters(slice)?;
                    let message = Message::ControlChange(ControlChangeMessage {
                        control: parameters[0],
                        value: parameters[1],
                    });
                    (slice, message)
                },
                0xC0 => {
                    // program change
                    // one data byte
                    let (slice, parameters): (_, [U7; 1]) = take_parameters(slice)?;
                    let message = Message::ProgramChange(ProgramChangeMessage {
                        program: parameters[0],
                    });
                    (slice, message)
                },
                0xD0 => {
                    // channel pressure
                    // one data byte
                    let (slice, parameters): (_, [U7; 1]) = take_parameters(slice)?;
                    let message = Message::ChannelPressure(ChannelValueMessage {
                        value: parameters[0],
                    });
                    (slice, message)
                },
                0xE0 => {
                    // pitch bend
                    // two data bytes
                    let (slice, parameters): (_, [U7; 2]) = take_parameters(slice)?;
                    let lsb = u16::from(u8::from(parameters[0]));
                    let msb = u16::from(u8::from(parameters[1]));
                    let value_u16 = (msb << 7) | lsb;
                    let value = U14::try_from(value_u16).unwrap();
                    let message = Message::PitchBend(PitchBendMessage {
                        value,
                    });
                    (slice, message)
                },
                0x00..=0x7F|0x81..=0x8F|0x91..=0x9F|0xA1..=0xAF
                        |0xB1..=0xBF|0xC1..=0xCF|0xD1..=0xDF|0xE1..=0xEF
                        |0xF0..=0xFF => unreachable!(),
            };
            (slice, EventData::Midi(MidiEventData { channel, message }))
        },
        0xF0|0xF7 => {
            // SysEx or SysEx continuation message
            // followed by a variable-length quantity for the data length
            let (slice, length_u32) = take_variable_length_quantity(slice)?;
            let length: usize = length_u32.try_into().unwrap();
            if length > slice.len() {
                return Err(Error::MoreLengthThanData);
            }
            let (sysex_slice, slice) = slice.split_at(length);
            let sysex_data = SysExEventData {
                data: sysex_slice.to_vec(),
            };
            let data = match status_byte {
                0xF0 => EventData::SysEx(sysex_data),
                0xF7 => EventData::RawSysEx(sysex_data),
                _ => unreachable!(),
            };
            (slice, data)
        },
        0xF1 => {
            // system message: time code
            let (slice, params): (_, [U7; 1]) = take_parameters(slice)?;
            let data: u8 = params[0].into();
            let message_type = U3::try_from((data >> 4) & 0b111).unwrap();
            let values = U4::try_from(data & 0b1111).unwrap();
            let event_data = EventData::System(SystemEventData::TimeCode(TimeCodeData {
                message_type,
                data: values,
            }));
            (slice, event_data)
        },
        0xF2 => {
            // system message: song position pointer
            let (slice, params): (_, [U7; 2]) = take_parameters(slice)?;
            let lsb: u8 = params[0].into();
            let msb: u8 = params[1].into();
            let pointer_u16 = (u16::from(msb) << 7) | u16::from(lsb);
            let pointer = U14::try_from(pointer_u16).unwrap();
            let event_data = EventData::System(SystemEventData::SongPosition(SongPositionData {
                position: pointer,
            }));
            (slice, event_data)
        },
        0xF3 => {
            // system message: song select
            let (slice, params): (_, [U7; 1]) = take_parameters(slice)?;
            let event_data = EventData::System(SystemEventData::SongSelect(SongSelectData {
                song: params[0],
            }));
            (slice, event_data)
        },
        0xF4|0xF5 => {
            // undefined message type
            return Err(Error::UnknownMessage { status_byte });
        },
        0xF6 => {
            // system message: tune request
            let event_data = EventData::System(SystemEventData::TuneRequest);
            (slice, event_data)
        },
        0xF8..=0xFE => {
            // real-time message
            let event_data = match status_byte {
                0xF8 => SystemEventData::TuneRequest,
                0xF9 => SystemEventData::F9,
                0xFA => SystemEventData::Start,
                0xFB => SystemEventData::Continue,
                0xFC => SystemEventData::Stop,
                0xFD => SystemEventData::FD,
                0xFE => SystemEventData::ActiveSensing,
                _ => unreachable!(),
            };
            let event_data = EventData::System(event_data);
            (slice, event_data)
        },
        0xFF => {
            // meta event
            // (0xFF is also used for System Reset;
            // in a Standard MIDI File, this must be encapsulated in 0xF7)
            let (slice, params): (_, [U7; 1]) = take_parameters(slice)?;
            let (slice, length_u32) = take_variable_length_quantity(slice)?;
            let length: usize = length_u32.try_into().unwrap();
            if length > slice.len() {
                return Err(Error::MoreLengthThanData);
            }
            let (meta_slice, slice) = slice.split_at(length);
            let data = EventData::Meta(MetaEventData {
                meta_type: params[0],
                data: meta_slice.to_owned(),
            });
            (slice, data)
        },
    };
    let event = Event {
        delta_time,
        data: event_data,
    };
    Ok((slice, event))
}

fn take_variable_length_quantity(slice: &[u8]) -> Result<(&[u8], u32), Error> {
    let mut value: u32 = 0;
    for (i, &b) in slice.iter().enumerate() {
        let byte_value = b & 0b0111_1111;
        if let Some(product) = value.checked_mul(0b1000_0000) {
            value = product;
        } else {
            return Err(Error::VariableLengthQuantityTooLarge {
                maximum: u32::MAX.into(),
                obtained: u64::from(value) * 0b1000_0000,
            });
        }
        value += u32::from(byte_value);

        let continue_flag = b & 0b1000_0000;
        if continue_flag == 0 {
            return Ok((&slice[(i+1)..], value));
        }
    }

    // we never saw a low byte and we ran out of bytes
    Err(Error::UnterminatedVariableLengthQuantity)
}

fn take_parameters<const COUNT: usize>(slice: &[u8]) -> Result<(&[u8], [U7; COUNT]), Error> {
    if slice.len() >= COUNT {
        let mut ret = [U7::zero(); COUNT];
        for (r, s) in ret.iter_mut().zip(slice.iter()) {
            match U7::try_from(*s) {
                Ok(u) => *r = u,
                Err(_) => return Err(Error::ParameterHighBitSet),
            }
        }
        Ok((&slice[COUNT..], ret))
    } else {
        Err(Error::NotEnoughParameters)
    }
}

pub fn write_smf<W: Write>(smf: &StandardMidiFile, writer: &mut W) -> Result<(), Error> {
    if usize::from(smf.header.track_count) != smf.tracks.len() {
        return Err(Error::TrackCountMismatch { header: smf.header.track_count, file: smf.tracks.len() });
    }

    write_smf_header(&smf.header, writer)?;

    for track in &smf.tracks {
        write_smf_track(track, writer)?;
    }

    Ok(())
}

fn write_smf_header<W: Write>(header: &FileHeader, writer: &mut W) -> Result<(), Error> {
    writer.write_all(b"MThd")?;
    let header_length_bytes: [u8; 4] = (6u32).to_be_bytes();
    writer.write_all(&header_length_bytes)?;
    let format_u16 = header.format.to_base_type();
    writer.write_all(&format_u16.to_be_bytes())?;
    writer.write_all(&header.track_count.to_be_bytes())?;
    writer.write_all(&header.division.to_be_bytes())?;
    Ok(())
}

fn write_smf_track<W: Write>(track: &Track, writer: &mut W) -> Result<(), Error> {
    let mut events_buf = Vec::new();

    for event in &track.events {
        write_smf_event(event, &mut events_buf)?;
    }

    writer.write_all(b"MTrk")?;
    let track_length_bytes: [u8; 4] = u32::try_from(events_buf.len()).unwrap().to_be_bytes();
    writer.write_all(&track_length_bytes)?;
    writer.write_all(&events_buf)?;
    Ok(())
}

fn write_smf_event<W: Write>(event: &Event, writer: &mut W) -> Result<(), Error> {
    write_variable_length_quantity(event.delta_time, writer)?;
    match &event.data {
        EventData::Midi(midi_event_data) => {
            match &midi_event_data.message {
                Message::NoteOff(note_message) => {
                    let buf = [
                        0x80 | u8::from(midi_event_data.channel),
                        note_message.note.into(),
                        note_message.value.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                Message::NoteOn(note_message) => {
                    let buf = [
                        0x90 | u8::from(midi_event_data.channel),
                        note_message.note.into(),
                        note_message.value.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                Message::KeyPressure(note_message) => {
                    let buf = [
                        0xA0 | u8::from(midi_event_data.channel),
                        note_message.note.into(),
                        note_message.value.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                Message::ControlChange(control_change_message) => {
                    let buf = [
                        0xB0 | u8::from(midi_event_data.channel),
                        control_change_message.control.into(),
                        control_change_message.value.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                Message::ProgramChange(program_change_message) => {
                    let buf = [
                        0xC0 | u8::from(midi_event_data.channel),
                        program_change_message.program.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                Message::ChannelPressure(channel_value_message) => {
                    let buf = [
                        0xD0 | u8::from(midi_event_data.channel),
                        channel_value_message.value.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                Message::PitchBend(pitch_bend_message) => {
                    let (lsb, msb) = pitch_bend_message.value.to_lsb_msb();
                    let buf = [
                        0xE0 | u8::from(midi_event_data.channel),
                        lsb.into(),
                        msb.into(),
                    ];
                    writer.write_all(&buf)?;
                },
            }
        },
        EventData::System(system_event_data) => {
            match system_event_data {
                SystemEventData::TimeCode(time_code_data) => {
                    let message_type = u8::from(time_code_data.message_type) << 4;
                    let data = u8::from(time_code_data.data) << 0;
                    let buf = [
                        0xF1,
                        message_type | data,
                    ];
                    writer.write_all(&buf)?;
                },
                SystemEventData::SongPosition(song_position_data) => {
                    let (lsb, msb) = song_position_data.position.to_lsb_msb();
                    let buf = [
                        0xF2,
                        lsb.into(),
                        msb.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                SystemEventData::SongSelect(song_select_data) => {
                    let buf = [
                        0xF3,
                        song_select_data.song.into(),
                    ];
                    writer.write_all(&buf)?;
                },
                SystemEventData::TuneRequest => {
                    writer.write_all(&[0xF6])?;
                },
                SystemEventData::TimingClock => {
                    writer.write_all(&[0xF8])?;
                },
                SystemEventData::F9 => {
                    writer.write_all(&[0xF9])?;
                },
                SystemEventData::Start => {
                    writer.write_all(&[0xFA])?;
                },
                SystemEventData::Continue => {
                    writer.write_all(&[0xFB])?;
                },
                SystemEventData::Stop => {
                    writer.write_all(&[0xFC])?;
                },
                SystemEventData::FD => {
                    writer.write_all(&[0xFD])?;
                },
                SystemEventData::ActiveSensing => {
                    writer.write_all(&[0xFE])?;
                },
                SystemEventData::SystemReset => {
                    writer.write_all(&[0xFF])?;
                },
            }
        },
        EventData::SysEx(sys_ex_event_data) => {
            writer.write_all(&[0xF0])?;
            write_variable_length_quantity(sys_ex_event_data.data.len().try_into().unwrap(), writer)?;
            writer.write_all(&sys_ex_event_data.data)?;
        },
        EventData::RawSysEx(sys_ex_event_data) => {
            writer.write_all(&[0xF7])?;
            write_variable_length_quantity(sys_ex_event_data.data.len().try_into().unwrap(), writer)?;
            writer.write_all(&sys_ex_event_data.data)?;
        },
        EventData::Meta(meta_event_data) => {
            let buf = [
                0xFF,
                meta_event_data.meta_type.into(),
            ];
            writer.write_all(&buf)?;
            write_variable_length_quantity(meta_event_data.data.len().try_into().unwrap(), writer)?;
            writer.write_all(&meta_event_data.data)?;
        },
    }
    Ok(())
}

fn write_variable_length_quantity<W: Write>(mut quantity: u32, writer: &mut W) -> Result<(), io::Error> {
    let mut buf = [0u8; 5];
    if quantity == 0 {
        // single zero byte
        return writer.write_all(&buf[0..1]);
    }

    let mut i = 0;
    while quantity > 0 {
        let value = u8::try_from(quantity & 0b0111_1111).unwrap();
        buf[i] = value;

        quantity >>= 7;
        i += 1;
    }

    // reverse it for big endian
    buf[0..i].reverse();

    // set continuation bits for all except the last byte
    for b in &mut buf[0..(i-1)] {
        *b |= 0b1000_0000;
    }

    // write it out
    writer.write_all(&buf[0..i])
}



#[cfg(test)]
mod tests {
    use super::{Error, take_variable_length_quantity, write_variable_length_quantity};

    #[test]
    fn test_take_variable_length_quantity() {
        fn tvlq(slice: &[u8]) -> u32 {
            let (slice, value) = take_variable_length_quantity(slice).unwrap();
            assert_eq!(slice.len(), 0);
            value
        }

        assert_eq!(tvlq(&[0x00]), 0x00);
        assert_eq!(tvlq(&[0x01]), 0x01);
        assert_eq!(tvlq(&[0x7E]), 0x7E);
        assert_eq!(tvlq(&[0x7F]), 0x7F);
        assert_eq!(tvlq(&[0x81, 0x00]), 0x80);
        assert_eq!(tvlq(&[0x81, 0x01]), 0x81);
        assert_eq!(tvlq(&[0x81, 0x02]), 0x82);
        assert_eq!(tvlq(&[0x81, 0x7E]), 0xFE);
        assert_eq!(tvlq(&[0x81, 0x7F]), 0xFF);
        assert_eq!(tvlq(&[0x82, 0x00]), 0x100);
        assert_eq!(tvlq(&[0x82, 0x01]), 0x101);
        assert_eq!(tvlq(&[0xFF, 0xFF, 0xFF, 0x7F]), 0x0FFF_FFFF);
        assert_eq!(tvlq(&[0x81, 0x80, 0x80, 0x80, 0x00]), 0x1000_0000);
        assert_eq!(tvlq(&[0x8F, 0xFF, 0xFF, 0xFF, 0x7F]), 0xFFFF_FFFF);

        if let Err(Error::VariableLengthQuantityTooLarge { maximum, obtained }) = take_variable_length_quantity(&[0x90, 0x80, 0x80, 0x80, 0x01]) {
            assert_eq!(maximum, 0xFFFF_FFFF);
            assert_eq!(obtained, 0x1_0000_0000);
        } else {
            panic!("take_variable_length_quantity returned unexpected success");
        }
    }

    #[test]
    fn test_write_variable_length_quantity() {
        fn wvlq(value: u32, slice: &[u8]) {
            let mut buf = Vec::new();
            write_variable_length_quantity(value, &mut buf).unwrap();
            assert_eq!(&buf, slice);
        }

        wvlq(0x00, &[0x00]);
        wvlq(0x01, &[0x01]);
        wvlq(0x7E, &[0x7E]);
        wvlq(0x7F, &[0x7F]);
        wvlq(0x80, &[0x81, 0x00]);
        wvlq(0x81, &[0x81, 0x01]);
        wvlq(0x82, &[0x81, 0x02]);
        wvlq(0xFE, &[0x81, 0x7E]);
        wvlq(0xFF, &[0x81, 0x7F]);
        wvlq(0x100, &[0x82, 0x00]);
        wvlq(0x101, &[0x82, 0x01]);
        wvlq(0x0FFF_FFFF, &[0xFF, 0xFF, 0xFF, 0x7F]);
        wvlq(0x1000_0000, &[0x81, 0x80, 0x80, 0x80, 0x00]);
        wvlq(0xFFFF_FFFF, &[0x8F, 0xFF, 0xFF, 0xFF, 0x7F]);

        if let Err(Error::VariableLengthQuantityTooLarge { maximum, obtained }) = take_variable_length_quantity(&[0x90, 0x80, 0x80, 0x80, 0x01]) {
            assert_eq!(maximum, 0xFFFF_FFFF);
            assert_eq!(obtained, 0x1_0000_0000);
        } else {
            panic!("take_variable_length_quantity returned unexpected success");
        }
    }
}
