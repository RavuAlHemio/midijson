use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

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
            pub const fn to_base_type(&self) -> $subtype {
                self.0
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

        impl TryFrom<u64> for $name {
            type Error = u64;
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                if value > $max_val {
                    Err(value)
                } else {
                    Ok(Self(value as $subtype))
                }
            }
        }
        impl From<$name> for u64 {
            fn from(value: $name) -> u64 {
                <$subtype>::from(value).into()
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
    pub meta_type: MetaType,
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
    pub control: Control,
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[repr(u8)]
pub enum Control {
    BankSelectMsb = 0,
    ModulationMsb = 1,
    BreathMsb = 2,
    // 3 undefined
    FootMsb = 4,
    PortamentoTimeMsb = 5,
    DataEntryMsb = 6,
    ChannelVolumeMsb = 7,
    BalanceMsb = 8,
    // 9 undefined
    PanMsb = 10,
    ExpressionMsb = 11,
    Effect1Msb = 12,
    Effect2Msb = 13,
    // 14-15 undefined
    GeneralPurpose1Msb = 16,
    GeneralPurpose2Msb = 17,
    GeneralPurpose3Msb = 18,
    GeneralPurpose4Msb = 19,
    // 20-31 undefined
    BankSelectLsb = 32,
    ModulationLsb = 33,
    BreathLsb = 34,
    // 35 undefined
    FootLsb = 36,
    PortamentoTimeLsb = 37,
    DataEntryLsb = 38,
    ChannelVolumeLsb = 39,
    BalanceLsb = 40,
    // 41 undefined
    PanLsb = 42,
    ExpressionLsb = 43,
    Effect1Lsb = 44,
    Effect2Lsb = 45,
    // 46-47 undefined
    GeneralPurpose1Lsb = 48,
    GeneralPurpose2Lsb = 49,
    GeneralPurpose3Lsb = 50,
    GeneralPurpose4Lsb = 51,
    // 52-63 undefined
    Sustain = 64,
    Portamento = 65,
    Sostenuto = 66,
    SoftPedal = 67,
    Legato = 68,
    Hold2 = 69,
    SoundController1 = 70,
    SoundController2 = 71,
    SoundController3 = 72,
    SoundController4 = 73,
    SoundController5 = 74,
    SoundController6 = 75,
    SoundController7 = 76,
    SoundController8 = 77,
    SoundController9 = 78,
    SoundController10 = 79,
    GeneralPurpose5 = 80,
    GeneralPurpose6 = 81,
    GeneralPurpose7 = 82,
    GeneralPurpose8 = 83,
    PortamentoControl = 84,
    // 85-90 undefined
    Effects1Depth = 91,
    Effects2Depth = 92,
    Effects3Depth = 93,
    Effects4Depth = 94,
    Effects5Depth = 95,
    DataIncrement = 96,
    DataDecrement = 97,
    NonRegisteredParameterNumberLsb = 98,
    NonRegisteredParameterNumberMsb = 99,
    RegisteredParameterNumberLsb = 100,
    RegisteredParameterNumberMsb = 101,
    // 102-119 undefined
    // now, the channel mode messages
    AllSoundOff = 120,
    ResetAllControllers = 121,
    LocalControl = 122,
    AllNotesOff = 123,
    OmniModeOff = 124,
    OmniModeOn = 125,
    MonoModeOn = 126,
    PolyModeOn = 127,
    // 128-255 not allowed
    Other(U7),
}
impl Control {
    pub const fn from_base_type(value: U7) -> Self {
        match value.to_base_type() {
            0 => Self::BankSelectMsb,
            1 => Self::ModulationMsb,
            2 => Self::BreathMsb,
            4 => Self::FootMsb,
            5 => Self::PortamentoTimeMsb,
            6 => Self::DataEntryMsb,
            7 => Self::ChannelVolumeMsb,
            8 => Self::BalanceMsb,
            10 => Self::PanMsb,
            11 => Self::ExpressionMsb,
            12 => Self::Effect1Msb,
            13 => Self::Effect2Msb,
            16 => Self::GeneralPurpose1Msb,
            17 => Self::GeneralPurpose2Msb,
            18 => Self::GeneralPurpose3Msb,
            19 => Self::GeneralPurpose4Msb,
            32 => Self::BankSelectLsb,
            33 => Self::ModulationLsb,
            34 => Self::BreathLsb,
            36 => Self::FootLsb,
            37 => Self::PortamentoTimeLsb,
            38 => Self::DataEntryLsb,
            39 => Self::ChannelVolumeLsb,
            40 => Self::BalanceLsb,
            42 => Self::PanLsb,
            43 => Self::ExpressionLsb,
            44 => Self::Effect1Lsb,
            45 => Self::Effect2Lsb,
            48 => Self::GeneralPurpose1Lsb,
            49 => Self::GeneralPurpose2Lsb,
            50 => Self::GeneralPurpose3Lsb,
            51 => Self::GeneralPurpose4Lsb,
            64 => Self::Sustain,
            65 => Self::Portamento,
            66 => Self::Sostenuto,
            67 => Self::SoftPedal,
            68 => Self::Legato,
            69 => Self::Hold2,
            70 => Self::SoundController1,
            71 => Self::SoundController2,
            72 => Self::SoundController3,
            73 => Self::SoundController4,
            74 => Self::SoundController5,
            75 => Self::SoundController6,
            76 => Self::SoundController7,
            77 => Self::SoundController8,
            78 => Self::SoundController9,
            79 => Self::SoundController10,
            80 => Self::GeneralPurpose5,
            81 => Self::GeneralPurpose6,
            82 => Self::GeneralPurpose7,
            83 => Self::GeneralPurpose8,
            84 => Self::PortamentoControl,
            91 => Self::Effects1Depth,
            92 => Self::Effects2Depth,
            93 => Self::Effects3Depth,
            94 => Self::Effects4Depth,
            95 => Self::Effects5Depth,
            96 => Self::DataIncrement,
            97 => Self::DataDecrement,
            98 => Self::NonRegisteredParameterNumberLsb,
            99 => Self::NonRegisteredParameterNumberMsb,
            100 => Self::RegisteredParameterNumberLsb,
            101 => Self::RegisteredParameterNumberMsb,
            120 => Self::AllSoundOff,
            121 => Self::ResetAllControllers,
            122 => Self::LocalControl,
            123 => Self::AllNotesOff,
            124 => Self::OmniModeOff,
            125 => Self::OmniModeOn,
            126 => Self::MonoModeOn,
            127 => Self::PolyModeOn,
            128..=255 => unreachable!(),
            _ => Self::Other(value),
        }
    }

    pub const fn to_base_type(&self) -> U7 {
        match self {
            Self::BankSelectMsb => U7::from_base_type(0),
            Self::ModulationMsb => U7::from_base_type(1),
            Self::BreathMsb => U7::from_base_type(2),
            Self::FootMsb => U7::from_base_type(4),
            Self::PortamentoTimeMsb => U7::from_base_type(5),
            Self::DataEntryMsb => U7::from_base_type(6),
            Self::ChannelVolumeMsb => U7::from_base_type(7),
            Self::BalanceMsb => U7::from_base_type(8),
            Self::PanMsb => U7::from_base_type(10),
            Self::ExpressionMsb => U7::from_base_type(11),
            Self::Effect1Msb => U7::from_base_type(12),
            Self::Effect2Msb => U7::from_base_type(13),
            Self::GeneralPurpose1Msb => U7::from_base_type(16),
            Self::GeneralPurpose2Msb => U7::from_base_type(17),
            Self::GeneralPurpose3Msb => U7::from_base_type(18),
            Self::GeneralPurpose4Msb => U7::from_base_type(19),
            Self::BankSelectLsb => U7::from_base_type(32),
            Self::ModulationLsb => U7::from_base_type(33),
            Self::BreathLsb => U7::from_base_type(34),
            Self::FootLsb => U7::from_base_type(36),
            Self::PortamentoTimeLsb => U7::from_base_type(37),
            Self::DataEntryLsb => U7::from_base_type(38),
            Self::ChannelVolumeLsb => U7::from_base_type(39),
            Self::BalanceLsb => U7::from_base_type(40),
            Self::PanLsb => U7::from_base_type(42),
            Self::ExpressionLsb => U7::from_base_type(43),
            Self::Effect1Lsb => U7::from_base_type(44),
            Self::Effect2Lsb => U7::from_base_type(45),
            Self::GeneralPurpose1Lsb => U7::from_base_type(48),
            Self::GeneralPurpose2Lsb => U7::from_base_type(49),
            Self::GeneralPurpose3Lsb => U7::from_base_type(50),
            Self::GeneralPurpose4Lsb => U7::from_base_type(51),
            Self::Sustain => U7::from_base_type(64),
            Self::Portamento => U7::from_base_type(65),
            Self::Sostenuto => U7::from_base_type(66),
            Self::SoftPedal => U7::from_base_type(67),
            Self::Legato => U7::from_base_type(68),
            Self::Hold2 => U7::from_base_type(69),
            Self::SoundController1 => U7::from_base_type(70),
            Self::SoundController2 => U7::from_base_type(71),
            Self::SoundController3 => U7::from_base_type(72),
            Self::SoundController4 => U7::from_base_type(73),
            Self::SoundController5 => U7::from_base_type(74),
            Self::SoundController6 => U7::from_base_type(75),
            Self::SoundController7 => U7::from_base_type(76),
            Self::SoundController8 => U7::from_base_type(77),
            Self::SoundController9 => U7::from_base_type(78),
            Self::SoundController10 => U7::from_base_type(79),
            Self::GeneralPurpose5 => U7::from_base_type(80),
            Self::GeneralPurpose6 => U7::from_base_type(81),
            Self::GeneralPurpose7 => U7::from_base_type(82),
            Self::GeneralPurpose8 => U7::from_base_type(83),
            Self::PortamentoControl => U7::from_base_type(84),
            Self::Effects1Depth => U7::from_base_type(91),
            Self::Effects2Depth => U7::from_base_type(92),
            Self::Effects3Depth => U7::from_base_type(93),
            Self::Effects4Depth => U7::from_base_type(94),
            Self::Effects5Depth => U7::from_base_type(95),
            Self::DataIncrement => U7::from_base_type(96),
            Self::DataDecrement => U7::from_base_type(97),
            Self::NonRegisteredParameterNumberLsb => U7::from_base_type(98),
            Self::NonRegisteredParameterNumberMsb => U7::from_base_type(99),
            Self::RegisteredParameterNumberLsb => U7::from_base_type(100),
            Self::RegisteredParameterNumberMsb => U7::from_base_type(101),
            Self::AllSoundOff => U7::from_base_type(120),
            Self::ResetAllControllers => U7::from_base_type(121),
            Self::LocalControl => U7::from_base_type(122),
            Self::AllNotesOff => U7::from_base_type(123),
            Self::OmniModeOff => U7::from_base_type(124),
            Self::OmniModeOn => U7::from_base_type(125),
            Self::MonoModeOn => U7::from_base_type(126),
            Self::PolyModeOn => U7::from_base_type(127),
            Self::Other(value) => *value,
        }
    }
}
impl PartialEq for Control {
    fn eq(&self, other: &Self) -> bool {
        self.to_base_type() == other.to_base_type()
    }
}
impl Eq for Control {
}
impl Ord for Control {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_base_type().cmp(&other.to_base_type())
    }
}
impl PartialOrd for Control {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Hash for Control {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_base_type().hash(state);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[repr(u8)]
pub enum MetaType {
    SequenceNumber = 0,
    TextEvent = 1,
    Copyright = 2,
    SequenceOrTrackName = 3,
    InstrumentName = 4,
    Lyric = 5,
    Marker = 6,
    CuePoint = 7,
    ProgramName = 8,
    DeviceName = 9,
    ChannelPrefix = 0x20,
    LegacyPort = 0x21,
    EndOfTrack = 0x2F,
    SetTempo = 0x51,
    SmpteOffset = 0x54,
    TimeSignature = 0x58,
    KeySignature = 0x59,
    SequencerSpecific = 0x7F,
    Other(U7),
}
impl MetaType {
    pub const fn from_base_type(value: U7) -> Self {
        match value.to_base_type() {
            0 => Self::SequenceNumber,
            1 => Self::TextEvent,
            2 => Self::Copyright,
            3 => Self::SequenceOrTrackName,
            4 => Self::InstrumentName,
            5 => Self::Lyric,
            6 => Self::Marker,
            7 => Self::CuePoint,
            8 => Self::ProgramName,
            9 => Self::DeviceName,
            0x20 => Self::ChannelPrefix,
            0x21 => Self::LegacyPort,
            0x2F => Self::EndOfTrack,
            0x51 => Self::SetTempo,
            0x54 => Self::SmpteOffset,
            0x58 => Self::TimeSignature,
            0x59 => Self::KeySignature,
            0x7F => Self::SequencerSpecific,
            128..=255 => unreachable!(),
            _ => Self::Other(value),
        }
    }

    pub const fn to_base_type(&self) -> U7 {
        match self {
            Self::SequenceNumber => U7::from_base_type(0),
            Self::TextEvent => U7::from_base_type(1),
            Self::Copyright => U7::from_base_type(2),
            Self::SequenceOrTrackName => U7::from_base_type(3),
            Self::InstrumentName => U7::from_base_type(4),
            Self::Lyric => U7::from_base_type(5),
            Self::Marker => U7::from_base_type(6),
            Self::CuePoint => U7::from_base_type(7),
            Self::ProgramName => U7::from_base_type(8),
            Self::DeviceName => U7::from_base_type(9),
            Self::ChannelPrefix => U7::from_base_type(0x20),
            Self::LegacyPort => U7::from_base_type(0x21),
            Self::EndOfTrack => U7::from_base_type(0x2F),
            Self::SetTempo => U7::from_base_type(0x51),
            Self::SmpteOffset => U7::from_base_type(0x54),
            Self::TimeSignature => U7::from_base_type(0x58),
            Self::KeySignature => U7::from_base_type(0x59),
            Self::SequencerSpecific => U7::from_base_type(0x7F),
            Self::Other(value) => *value,
        }
    }
}
impl PartialEq for MetaType {
    fn eq(&self, other: &Self) -> bool {
        self.to_base_type() == other.to_base_type()
    }
}
impl Eq for MetaType {
}
impl Ord for MetaType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_base_type().cmp(&other.to_base_type())
    }
}
impl PartialOrd for MetaType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Hash for MetaType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_base_type().hash(state);
    }
}

pub trait KnownMaxValue {
    fn known_max_value() -> Self;
}

macro_rules! impl_primitive_max_value {
    ($type:ty) => {
        impl KnownMaxValue for $type {
            fn known_max_value() -> Self { Self::MAX }
        }
    }
}
impl_primitive_max_value!(u8);
impl_primitive_max_value!(u16);
impl_primitive_max_value!(u32);
impl KnownMaxValue for U3 {
    fn known_max_value() -> Self {
        U3::from_base_type(0b0111)
    }
}
impl KnownMaxValue for U4 {
    fn known_max_value() -> Self {
        U4::from_base_type(0b1111)
    }
}
impl KnownMaxValue for U7 {
    fn known_max_value() -> Self {
        U7::from_base_type(0b0111_1111)
    }
}
impl KnownMaxValue for U14 {
    fn known_max_value() -> Self {
        U14::from_base_type(0b0011_1111_1111_1111)
    }
}
