use serde::{Deserialize, Serialize};

use crate::{ControlFunction, Error, Note, ToSliceError, U14, U7};
use core::convert::TryFrom;

use std::io;

/// Holds information based on the Midi 1.0 spec.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiMessage {
    /// This message is sent when a note is released (ended).
    NoteOff(Channel, Note, Velocity),

    /// This message is sent when a note is depressed (start).
    NoteOn(Channel, Note, Velocity),

    /// This message is most often sent by pressing down on the key after it "bottoms out".
    PolyphonicKeyPressure(Channel, Note, Velocity),

    /// This message is sent when a controller value changes. Controllers include devices such as pedals and levers.
    ///
    /// Controller numbers 120-127 are reserved as "Channel Mode Messages".
    ControlChange(Channel, ControlFunction, ControlValue),

    /// This message is sent when the patch number changes.
    ProgramChange(Channel, ProgramNumber),

    /// This message is most often sent by pressing down on the key after it "bottoms out". This message is different
    /// from polyphonic after-touch. Use this message to send the single greatest pressure value (of all the current
    /// depressed keys).
    ChannelPressure(Channel, Velocity),

    /// This message is sent to indicate a change in the pitch bender (wheel or level, typically). The pitch bender is
    /// measured by a fourteen bit value. Center is 8192.
    PitchBendChange(Channel, PitchBend),

    /// A sysex message. The data bytes are not stored to improve performance.
    SysEx,

    /// MIDI Time Code Quarter Frame.
    ///
    /// The data is in the format 0nnndddd where nnn is the Message Type and dddd is the Value.
    ///
    /// TODO: Interpret data instead of providing the raw format.
    MidiTimeCode(U7),

    /// This is an internal 14 bit value that holds the number of MIDI beats (1 beat = six MIDI clocks) since the start
    /// of the song.
    SongPositionPointer(SongPosition),

    /// The Song Select specifies which sequence or song is to be played.
    SongSelect(Song),

    /// The u8 data holds the status byte.
    Reserved(u8),

    /// Upon receiving a Tune Request, all analog synthesizers should tune their oscillators.
    TuneRequest,

    /// Timing Clock. Sent 24 times per quarter note when synchronization is required.
    TimingClock,

    /// Start the current sequence playing. (This message will be followed with Timing Clocks).
    Start,

    /// Continue at the point the sequence was Stopped.
    Continue,

    /// Stop the current sequence.
    Stop,

    /// This message is intended to be sent repeatedly to tell the receiver that a connection is alive. Use of this
    /// message is optional. When initially received, the receiver will expect to receive another Active Sensing message
    /// each 300ms (max), and if it idoes not, then it will assume that the connection has been terminated. At
    /// termination, the receiver will turn off all voices and return to normal (non-active sensing) operation.
    ActiveSensing,

    /// Reset all receivers in the system to power-up status. This should be used sparingly, preferably under manual
    /// control. In particular, it should not be sent on power-up.
    Reset,
}

impl TryFrom<&[u8]> for MidiMessage {
    type Error = Error;
    /// Construct a midi message from bytes.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            return Err(Error::NoBytes);
        }
        if !is_status_byte(bytes[0]) {
            return Err(Error::UnexpectedDataByte);
        }
        let chan = Channel::from_index(bytes[0] & 0x0F)?;
        let data_a = bytes
            .get(1)
            .ok_or(Error::NotEnoughBytes)
            .and_then(|b| valid_data_byte(*b));
        let data_b = bytes
            .get(2)
            .ok_or(Error::NotEnoughBytes)
            .and_then(|b| valid_data_byte(*b));
        match bytes[0] & 0xF0 {
            0x80 => Ok(MidiMessage::NoteOff(chan, Note::from(data_a?), data_b?)),
            0x90 => match data_b? {
                U7::MIN => Ok(MidiMessage::NoteOff(chan, Note::from(data_a?), U7::MIN)),
                _ => Ok(MidiMessage::NoteOn(chan, Note::from(data_a?), data_b?)),
            },
            0xA0 => Ok(MidiMessage::PolyphonicKeyPressure(
                chan,
                Note::from(data_a?),
                data_b?,
            )),
            0xB0 => Ok(MidiMessage::ControlChange(chan, data_a?.into(), data_b?)),
            0xC0 => Ok(MidiMessage::ProgramChange(chan, data_a?)),
            0xD0 => Ok(MidiMessage::ChannelPressure(chan, data_a?)),
            0xE0 => Ok(MidiMessage::PitchBendChange(
                chan,
                combine_data(data_a?, data_b?),
            )),
            0xF0 => match bytes[0] {
                // TODO: Parse for messages.
                0xF0 => Ok(MidiMessage::SysEx),
                0xF1 => Ok(MidiMessage::MidiTimeCode(data_a?)),
                0xF2 => Ok(MidiMessage::SongPositionPointer(combine_data(
                    data_a?, data_b?,
                ))),
                0xF3 => Ok(MidiMessage::SongSelect(data_a?)),
                0xF4 | 0xF5 => Ok(MidiMessage::Reserved(bytes[0])),
                0xF6 => Ok(MidiMessage::TuneRequest),
                0xF7 => Err(Error::UnexpectedEndSysExByte),
                0xF8 => Ok(MidiMessage::TimingClock),
                0xF9 => Ok(MidiMessage::Reserved(bytes[0])),
                0xFA => Ok(MidiMessage::Start),
                0xFB => Ok(MidiMessage::Continue),
                0xFC => Ok(MidiMessage::Stop),
                0xFD => Ok(MidiMessage::Reserved(bytes[0])),
                0xFE => Ok(MidiMessage::ActiveSensing),
                0xFF => Ok(MidiMessage::Reset),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

impl<'a> MidiMessage {
    /// Construct a midi message from bytes.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        MidiMessage::try_from(bytes)
    }

    /// Copies the message as bytes to slice. If slice does not have enough capacity to fit the
    /// message, then an error is returned. On success, the number of bytes written will be
    /// returned. This should be the same number obtained from `self.bytes_size()`.
    #[allow(clippy::range_plus_one)]
    pub fn copy_to_slice(&self, slice: &mut [u8]) -> Result<usize, ToSliceError> {
        if slice.len() < self.bytes_size() {
            Err(ToSliceError::BufferTooSmall)
        } else {
            let slice = &mut slice[..self.bytes_size()];
            match self {
                MidiMessage::NoteOff(a, b, c) => {
                    slice.copy_from_slice(&[0x80 | a.index(), u8::from(*b), u8::from(*c)]);
                }
                MidiMessage::NoteOn(a, b, c) => {
                    slice.copy_from_slice(&[0x90 | a.index(), u8::from(*b), u8::from(*c)]);
                }
                MidiMessage::PolyphonicKeyPressure(a, b, c) => {
                    slice.copy_from_slice(&[0xA0 | a.index(), *b as u8, u8::from(*c)]);
                }
                MidiMessage::ControlChange(a, b, c) => {
                    slice.copy_from_slice(&[0xB0 | a.index(), u8::from(*b), u8::from(*c)]);
                }
                MidiMessage::ProgramChange(a, b) => {
                    slice.copy_from_slice(&[0xC0 | a.index(), u8::from(*b)]);
                }
                MidiMessage::ChannelPressure(a, b) => {
                    slice.copy_from_slice(&[0xD0 | a.index(), u8::from(*b)]);
                }
                MidiMessage::PitchBendChange(a, b) => {
                    let (b1, b2) = split_data(*b);
                    slice.copy_from_slice(&[0xE0 | a.index(), b1, b2]);
                }
                MidiMessage::SysEx => {
                    slice[0] = 0xF0;
                    slice[1] = 0xF7;
                }
                MidiMessage::MidiTimeCode(a) => slice.copy_from_slice(&[0xF1, u8::from(*a)]),
                MidiMessage::SongPositionPointer(a) => {
                    let (a1, a2) = split_data(*a);
                    slice.copy_from_slice(&[0xF2, a1, a2]);
                }
                MidiMessage::SongSelect(a) => slice.copy_from_slice(&[0xF3, u8::from(*a)]),
                MidiMessage::Reserved(a) => slice.copy_from_slice(&[*a]),
                MidiMessage::TuneRequest => slice.copy_from_slice(&[0xF6]),
                MidiMessage::TimingClock => slice.copy_from_slice(&[0xF8]),
                MidiMessage::Start => slice.copy_from_slice(&[0xFA]),
                MidiMessage::Continue => slice.copy_from_slice(&[0xFB]),
                MidiMessage::Stop => slice.copy_from_slice(&[0xFC]),
                MidiMessage::ActiveSensing => slice.copy_from_slice(&[0xFE]),
                MidiMessage::Reset => slice.copy_from_slice(&[0xFF]),
            };
            Ok(self.bytes_size())
        }
    }

    /// The number of bytes the MIDI message takes when converted to bytes.
    pub fn bytes_size(&self) -> usize {
        match self {
            MidiMessage::NoteOff(..) => 3,
            MidiMessage::NoteOn(..) => 3,
            MidiMessage::PolyphonicKeyPressure(..) => 3,
            MidiMessage::ControlChange(..) => 3,
            MidiMessage::ProgramChange(..) => 2,
            MidiMessage::ChannelPressure(..) => 2,
            MidiMessage::PitchBendChange(..) => 3,
            // This is only true because `MidiMessage` throws away all the data bytes.
            MidiMessage::SysEx => 2,
            MidiMessage::MidiTimeCode(_) => 2,
            MidiMessage::SongPositionPointer(_) => 3,
            MidiMessage::SongSelect(_) => 2,
            MidiMessage::Reserved(_) => 1,
            MidiMessage::TuneRequest => 1,
            MidiMessage::TimingClock => 1,
            MidiMessage::Start => 1,
            MidiMessage::Continue => 1,
            MidiMessage::Stop => 1,
            MidiMessage::ActiveSensing => 1,
            MidiMessage::Reset => 1,
        }
    }

    /// The number of bytes the MIDI message takes when encoded with the `std::io::Read` trait.
    #[deprecated(
        since = "3.1.0",
        note = "Function has been renamed to MidiMessage::bytes_size()."
    )]
    pub fn wire_size(&self) -> usize {
        self.bytes_size()
    }

    /// The channel associated with the MIDI message, if applicable for the message type.
    pub fn channel(&self) -> Option<Channel> {
        match self {
            MidiMessage::NoteOff(c, ..) => Some(*c),
            MidiMessage::NoteOn(c, ..) => Some(*c),
            MidiMessage::PolyphonicKeyPressure(c, ..) => Some(*c),
            MidiMessage::ControlChange(c, ..) => Some(*c),
            MidiMessage::ProgramChange(c, ..) => Some(*c),
            MidiMessage::ChannelPressure(c, ..) => Some(*c),
            MidiMessage::PitchBendChange(c, ..) => Some(*c),
            _ => None,
        }
    }

    /// Convert the message to a vector of bytes. Prefer using
    /// `copy_to_slice` if possible for better performance.
    #[cfg(feature = "std")]
    pub fn to_vec(&self) -> Vec<u8> {
        let mut data = vec![0; self.bytes_size()];
        // Unwrapping is ok as data has enough capacity for the data.
        self.copy_to_slice(&mut data).unwrap();
        data
    }
}

impl io::Read for MidiMessage {
    // Use MidiMessage::copy_from_slice instead.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.copy_to_slice(buf) {
            Ok(n) => Ok(n),
            Err(ToSliceError::BufferTooSmall) => Ok(0),
        }
    }
}

/// Specifies the velocity of an action (often key press, release, or aftertouch).
pub type Velocity = U7;

/// Specifies the value of a MIDI control.
pub type ControlValue = U7;

/// Specifies a program. Sometimes known as patch.
pub type ProgramNumber = U7;

/// A 14bit value specifying the pitch bend. Neutral is 8192.
pub type PitchBend = U14;

/// 14 bit value that holds the number of MIDI beats (1 beat = six MIDI clocks) since the start of the song.
pub type SongPosition = U14;

/// A song or sequence.
pub type Song = U7;

/// The MIDI channel. There are 16 channels. They are numbered between 1 and 16
/// inclusive, or indexed between 0 and 15 inclusive.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Channel {
    Ch1,
    Ch2,
    Ch3,
    Ch4,
    Ch5,
    Ch6,
    Ch7,
    Ch8,
    Ch9,
    Ch10,
    Ch11,
    Ch12,
    Ch13,
    Ch14,
    Ch15,
    Ch16,
}

impl Channel {
    /// Get a MIDI channel from an index that is between 0 and 15 inclusive.
    pub fn from_index(i: u8) -> Result<Channel, Error> {
        match i {
            0 => Ok(Channel::Ch1),
            1 => Ok(Channel::Ch2),
            2 => Ok(Channel::Ch3),
            3 => Ok(Channel::Ch4),
            4 => Ok(Channel::Ch5),
            5 => Ok(Channel::Ch6),
            6 => Ok(Channel::Ch7),
            7 => Ok(Channel::Ch8),
            8 => Ok(Channel::Ch9),
            9 => Ok(Channel::Ch10),
            10 => Ok(Channel::Ch11),
            11 => Ok(Channel::Ch12),
            12 => Ok(Channel::Ch13),
            13 => Ok(Channel::Ch14),
            14 => Ok(Channel::Ch15),
            15 => Ok(Channel::Ch16),
            _ => Err(Error::ChannelOutOfRange),
        }
    }

    /// The index of this midi channel. The returned value is between 0 and 15
    /// inclusive.
    pub fn index(self) -> u8 {
        match self {
            Channel::Ch1 => 0,
            Channel::Ch2 => 1,
            Channel::Ch3 => 2,
            Channel::Ch4 => 3,
            Channel::Ch5 => 4,
            Channel::Ch6 => 5,
            Channel::Ch7 => 6,
            Channel::Ch8 => 7,
            Channel::Ch9 => 8,
            Channel::Ch10 => 9,
            Channel::Ch11 => 10,
            Channel::Ch12 => 11,
            Channel::Ch13 => 12,
            Channel::Ch14 => 13,
            Channel::Ch15 => 14,
            Channel::Ch16 => 15,
        }
    }

    /// The number of this midi channel. The returned value is between 1 and 16
    /// inclusive.
    pub fn number(self) -> u8 {
        self.index() + 1
    }
}

#[inline(always)]
fn combine_data(lower: U7, higher: U7) -> U14 {
    let raw = u16::from(u8::from(lower)) + 128 * u16::from(u8::from(higher));
    unsafe { U14::from_unchecked(raw) }
}

#[inline(always)]
fn split_data(data: U14) -> (u8, u8) {
    ((u16::from(data) % 128) as u8, (u16::from(data) / 128) as u8)
}

#[inline(always)]
fn is_status_byte(b: u8) -> bool {
    b & 0x80 == 0x80
}

#[inline(always)]
fn valid_data_byte(b: u8) -> Result<U7, Error> {
    U7::try_from(b).map_err(|_| Error::UnexpectedStatusByte)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ControlFunction, Error, Note};

    #[test]
    fn try_from() {
        assert_eq!(
            MidiMessage::try_from([].as_ref()),
            Err(Error::NoBytes),
            "no bytes produces an error",
        );
        assert_eq!(
            MidiMessage::try_from([0x00].as_ref()),
            Err(Error::UnexpectedDataByte),
            "no status byte produces an error",
        );
        assert_eq!(
            MidiMessage::try_from([0x84].as_ref()),
            Err(Error::NotEnoughBytes),
            "NoteOff event produces errors with only 1 byte",
        );
        assert_eq!(
            MidiMessage::try_from([0x84, 64].as_ref()),
            Err(Error::NotEnoughBytes),
            "NoteOff event produces errors with only 2 bytes",
        );
        assert_eq!(
            MidiMessage::try_from([0x84, 64, 100].as_ref()),
            Ok(MidiMessage::NoteOff(
                Channel::Ch5,
                Note::E4,
                U7::try_from(100).unwrap()
            )),
            "NoteOff event is decoded.",
        );

        assert_eq!(
            MidiMessage::try_from([0x94].as_ref()),
            Err(Error::NotEnoughBytes),
            "NoteOn event produces errors with only 1 byte",
        );
        assert_eq!(
            MidiMessage::try_from([0x94, 64].as_ref()),
            Err(Error::NotEnoughBytes),
            "NoteOn event produces errors with only 2 bytes",
        );
        assert_eq!(
            MidiMessage::try_from([0x94, 64, 100].as_ref()),
            Ok(MidiMessage::NoteOn(
                Channel::Ch5,
                Note::E4,
                U7::try_from(100).unwrap()
            )),
            "NoteOn event is decoded.",
        );
        assert_eq!(
            MidiMessage::try_from([0x94, 64, 0].as_ref()),
            Ok(MidiMessage::NoteOff(
                Channel::Ch5,
                Note::E4,
                U7::try_from(0).unwrap()
            )),
            "NoteOn message with 0 veloctiy decodes as NoteOff",
        );

        assert_eq!(
            MidiMessage::try_from([0xF0, 4, 8, 12, 16, 0xF7].as_ref()),
            Ok(MidiMessage::SysEx),
            "SysEx message is decoded and data bytes are thrown away.",
        );
        assert_eq!(
            MidiMessage::try_from([0xF0, 3, 6, 9, 12, 15, 0xF7, 125].as_ref()),
            Ok(MidiMessage::SysEx),
            "SysEx message does not include bytes after the end byte.",
        );
        // TODO: Handle this use case.
        // assert_eq!(
        //     MidiMessage::try_from([0xF0, 1, 2, 3, 4, 5, 6, 7, 8, 9].as_ref()),
        //     Err(Error::NoSysExEndByte),
        //     "SysEx message without end status produces error.",
        // );

        assert_eq!(
            MidiMessage::try_from([0xE4].as_ref()),
            Err(Error::NotEnoughBytes),
            "PitchBend with single byte produces error.",
        );
        assert_eq!(
            MidiMessage::try_from([0xE4, 64].as_ref()),
            Err(Error::NotEnoughBytes),
            "PitchBend with only 2 bytes produces error.",
        );
        assert_eq!(
            MidiMessage::try_from([0xE4, 64, 100].as_ref()),
            Ok(MidiMessage::PitchBendChange(
                Channel::Ch5,
                U14::try_from(12864).unwrap()
            )),
            "PitchBendChange is decoded.",
        );
    }

    #[test]
    fn copy_to_slice() {
        let b = {
            let mut b = [0u8; 6];
            let bytes_copied = MidiMessage::PolyphonicKeyPressure(
                Channel::Ch10,
                Note::A6,
                U7::try_from(43).unwrap(),
            )
            .copy_to_slice(&mut b)
            .unwrap();
            assert_eq!(bytes_copied, 3);
            b
        };
        assert_eq!(b, [0xA9, 93, 43, 0, 0, 0]);
    }

    #[test]
    fn copy_to_slice_sysex() {
        let b = {
            let mut b = [0u8; 8];
            let bytes_copied = MidiMessage::SysEx.copy_to_slice(&mut b).unwrap();
            assert_eq!(bytes_copied, 2);
            b
        };
        assert_eq!(b, [0xF0, 0xF7, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn channel() {
        assert_eq!(
            MidiMessage::ControlChange(
                Channel::Ch8,
                ControlFunction::DAMPER_PEDAL,
                U7::try_from(55).unwrap()
            )
            .channel(),
            Some(Channel::Ch8)
        );
        assert_eq!(MidiMessage::Start.channel(), None);
    }
}
