use from_to_repr::from_to_other;
use serde::{Deserialize, Serialize};


macro_rules! define_subint {
    ($name:ident, $subtype:ty, $max_val:expr) => {
        #[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        pub struct $name($subtype);
        impl $name {
            pub const fn zero() -> Self { Self(0) }
            pub const fn from_base_type(value: $subtype) -> Self {
                if value > $max_val {
                    panic!("value too large");
                } else {
                    Self(value)
                }
            }
        }
        impl TryFrom<$subtype> for $name {
            type Error = $subtype;
            fn try_from(value: $subtype) -> Result<Self, Self::Error> {
                if value > $max_val {
                    Err(value)
                } else {
                    Ok(Self(value))
                }
            }
        }
        impl From<$name> for $subtype {
            fn from(value: $name) -> $subtype {
                value.0
            }
        }
    };
}

define_subint!(U3, u8, 0b111);
define_subint!(U4, u8, 0b1111);
define_subint!(U7, u8, 0b111_1111);
define_subint!(U14, u16, 0b11_1111_1111_1111);

impl U14 {
    pub const fn to_lsb_msb(&self) -> (U7, U7) {
        let lsb_big = (self.0 >> 0) & 0b0111_1111;
        let msb_big = (self.0 >> 7) & 0b0111_1111;
        assert!(lsb_big < 0b1000_0000);
        assert!(msb_big < 0b1000_0000);

        let lsb = lsb_big as u8;
        assert!(lsb & 0b1000_0000 == 0);
        let msb = msb_big as u8;
        assert!(msb & 0b1000_0000 == 0);

        let lsb_u7 = U7::from_base_type(lsb);
        let msb_u7 = U7::from_base_type(msb);

        (lsb_u7, msb_u7)
    }
}


#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct StandardMidiFile {
    pub header: FileHeader,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FileHeader {
    pub format: SmfFormat,
    pub track_count: u16,
    pub division: i16,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum SmfFormat {
    SingleTrack = 0,
    MultiTrack = 1,
    MultiPattern = 2,
    Other(u16),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Track {
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Event {
    pub delta_time: u32,
    pub data: EventData,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type")]
pub enum EventData {
    Midi(MidiEventData),
    System(SystemEventData),
    SysEx(SysExEventData),
    RawSysEx(SysExEventData),
    Meta(MetaEventData),
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SysExEventData {
    // header: 0xF7 or 0xF0 depending on SysEx vs. RawSysEx
    // length: var_length_int,

    /// The data of the SysEx event, excluding the leading 0xF0 and the length but including the
    /// trailing 0xF7.
    ///
    /// If a SysEx event is to be split over time, [`EventData::SysExData`] and
    /// [`EventData::RawSysExData`] may be combined. The example in the Standard MIDI File 1.0
    /// specification is as follows:
    ///
    /// ```plain
    /// F0 03 43 12 00
    /// 81 48                    (200-tick delta time)
    /// F7 06 43 12 00 43 12 00
    /// 64                       (100-tick delta time)
    /// F7 04 43 12 00 F7
    /// ```
    ///
    /// This can be represented as:
    ///
    /// ```
    /// let track = Track {
    ///     events: vec![
    ///         Event {
    ///             delta_time: 0,
    ///             data: EventData::SysEx(SysExEventData {
    ///                 data: vec![0x43, 0x12, 0x00],
    ///             }),
    ///         },
    ///         Event {
    ///             delta_time: 200,
    ///             data: EventData::RawSysEx(SysExEventData {
    ///                 data: vec![0x43, 0x12, 0x00, 0x43, 0x12, 0x00],
    ///             }),
    ///         },
    ///         Event {
    ///             delta_time: 100,
    ///             data: EventData::RawSysEx(SysExEventData {
    ///                 data: vec![0x43, 0x12, 0x00, 0xF7],
    ///             }),
    ///         },
    ///     ],
    /// };
    /// ```
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct MetaEventData {
    pub meta_type: U7,
    // length: var_length_int,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct MidiEventData {
    // message_type: u4,
    pub channel: U4,
    pub message: Message,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type")]
pub enum Message {
    NoteOff(NoteMessage), // 0x8_
    NoteOn(NoteMessage), // 0x9_
    KeyPressure(NoteMessage), // 0xA_
    ControlChange(ControlChangeMessage), // 0xB_
    ProgramChange(ProgramChangeMessage), // 0xC_
    ChannelPressure(ChannelValueMessage), // 0xD_
    PitchBend(PitchBendMessage), // 0xE_
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type")]
pub enum SystemEventData {
    TimeCode(TimeCodeData), // 0xF1
    SongPosition(SongPositionData), // 0xF2
    SongSelect(SongSelectData), // 0xF3
    TuneRequest, // 0xF6, no data
    // 0xF7: end of SysEx; only appears at end of SysEx data
    TimingClock, // 0xF8, no data
    F9, // 0xF9, no data
    Start, // 0xFA, no data
    Continue, // 0xFB, no data
    Stop, // 0xFC, no data
    FD, // 0xFD, no data
    ActiveSensing, // 0xFE, no data
    SystemReset, // 0xFF, no data
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NoteMessage {
    pub note: U7,
    pub value: U7,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ControlChangeMessage {
    pub control: U7,
    pub value: U7,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ProgramChangeMessage {
    pub program: U7,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ChannelValueMessage {
    pub value: U7,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PitchBendMessage {
    pub value: U14,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TimeCodeData {
    pub message_type: U3,
    pub data: U4,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SongPositionData {
    pub position: U14,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SongSelectData {
    pub song: U7,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SystemData {
    pub data: Vec<u8>,
}
