mod byte;
mod cc;
mod error;
mod midi_message;
mod note;

pub use byte::{U14, U7};
pub use cc::ControlFunction;
pub use error::{FromBytesError, ToSliceError};
pub use midi_message::{
    Channel, ControlValue, MidiMessage, PitchBend, ProgramNumber, Song, SongPosition, Velocity,
};
pub use note::Note;

/// Use `FromBytesError` instead.
pub type Error = FromBytesError;
