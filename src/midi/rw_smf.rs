//! Reading and writing Standard MIDI Files.


use std::fmt;
use std::io::{self, Read, Write};

use crate::midi::model::{FileHeader, SmfFormat, StandardMidiFile};


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

    for _ in 0..header.track_count {
        // expect a track chunk
        reader.read_exact(&mut four_buf)?;
        Error::ensure_track_chunk(*b"MTrk", four_buf)?;

        reader.read_exact(&mut four_buf)?;
        let track_length = u32::from_be_bytes(four_buf);

        let track_length_usize = track_length.try_into().unwrap();
        let mut track_data = vec![0u8; track_length_usize];
        reader.read_exact(&mut track_data)?;

        todo!("crunch the track data");
    }

    todo!();
}
