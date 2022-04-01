mod sixel_event;

use arrayvec::{ArrayVec, CapacityError};
use std::num::ParseIntError;
use std::str::{self, Utf8Error};

use thiserror::Error;

use sixel_event::SixelEvent;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Failed to parse")]
    ParsingError,
    #[error("Failed to parse")]
    CapacityError(#[from] CapacityError<ArrayVec<u8, 3>>),
    #[error("Failed to parse")]
    Utf8Error(#[from] Utf8Error),
    #[error("Failed to parse")]
    ParseIntError(#[from] ParseIntError),
}

fn main() {
    // let sample = "#0;2;0;0;0#1;2;100;100;0#2;2;0;100;0";
    // let sample = "\u{1b}Pq#0;2;0;0;0#1;2;100;100;0#2;2;0;100;0";
    let sample = "\u{1b}Pq
    #0;2;0;0;0#1;2;100;100;0#2;2;0;100;0
    #1~~@@vv@@~~@@~~$
    #2??}}GG}}??}}??-
    #1!14@
    \u{1b}\\";
    let events = sixel_machine(sample);
    for event in events {
        println!("{:?}", event);
    }
}

fn sixel_machine(sample: &str) -> Vec<SixelEvent> {
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(byte, |sixel_event| events.push(sixel_event));
    }
    events
}

#[derive(Clone, Copy, Debug)]
pub enum ParserState {
    Ground,
    DeviceControlString,
    EscapeCharacter,
    ColorIntroducer,
    RasterAttribute,
    GraphicsRepeatIntroducer,
    UnknownInstruction,
}

pub struct Parser {
    state: ParserState,
    raw_instruction: ArrayVec<u8, 256>, // TODO: proper cap
    pending_event_fields: ArrayVec<ArrayVec<u8, 3>, 5>,
    currently_parsing: ArrayVec<u8, 256>, // TODO: proper cap
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            state: ParserState::Ground,
            raw_instruction: ArrayVec::new(),
            pending_event_fields: ArrayVec::new(),
            currently_parsing: ArrayVec::new(),
        }
    }
    pub fn advance(&mut self, byte: &u8, cb: impl FnMut(SixelEvent)) {
        if byte == &b' ' || byte == &b'\n' || byte == &b'\t' {
            // ignore whitespace
            return;
        }
        self.raw_instruction.push(*byte);
//         println!(
//             "byte: {:?}, current_state: {:?}",
//             str::from_utf8(&[*byte]),
//             self.state
//         );
        self.move_to_next_state(*byte, cb);
        if *byte == b';' {
            self.finalize_field().unwrap(); // TODO: not unwrap
        } else if let b'0'..=b'9' = byte {
            self.currently_parsing.push(*byte);
        }
    }
    fn move_to_next_state(&mut self, byte: u8, mut cb: impl FnMut(SixelEvent)) {
        let current_state = self.state;
        match (current_state, byte) {
            (ParserState::EscapeCharacter, b'P' | b'\\') => {
                self.state = ParserState::DeviceControlString;
            }
            (ParserState::DeviceControlString, b'q') => {
                cb(self.emit_dcs_event().unwrap()); // TODO: not unwrap
                self.state = ParserState::Ground;
            }
            (ParserState::GraphicsRepeatIntroducer, b'?'..=b'~') => {
                cb(self.emit_repeat_introducer_event(byte).unwrap()); // TODO: not unwrap
            }
            (_, b'?'..=b'~' | b'$' | b'-') => {
                self.emit_possible_pending_event(&mut cb); // TODO: not unwrap
                cb(self.emit_single_byte_event(byte).unwrap()); // TODO: not unwrap
                self.state = ParserState::Ground;
            }
            (_, b'#') => {
                self.emit_possible_pending_event(cb); // TODO: not unwrap
                self.state = ParserState::ColorIntroducer;
            }
            (_, b'"') => {
                self.emit_possible_pending_event(cb); // TODO: not unwrap
                self.state = ParserState::RasterAttribute;
            }
            (_, b'!') => {
                self.emit_possible_pending_event(cb); // TODO: not unwrap
                self.state = ParserState::GraphicsRepeatIntroducer;
            }
            (_, 27) => {
                self.emit_possible_pending_event(cb); // TODO: not unwrap
                self.state = ParserState::EscapeCharacter;
            }
            _ => {}
        };
    }
    fn emit_dcs_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        SixelEvent::new_dcs(&mut self.pending_event_fields)
        // TODO: clear raw
    }
    fn emit_color_introducer_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        SixelEvent::new_color_introducer(&mut self.pending_event_fields)
        // TODO: clear raw
    }
    fn emit_raster_attribute_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        SixelEvent::new_raster_attribute(&mut self.pending_event_fields)
        // TODO: clear raw
    }
    fn emit_sixel_data_event(&mut self, byte: u8) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        Ok(SixelEvent::Data { byte })
    }
    fn emit_repeat_introducer_event(&mut self, byte: u8) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        SixelEvent::new_repeat(&mut self.pending_event_fields, byte)
    }
    fn emit_beginning_of_line_event(&mut self) -> Result<SixelEvent, ParserError> {
        Ok(SixelEvent::GotoBeginningOfLine)
    }
    fn emit_next_line_event(&mut self) -> Result<SixelEvent, ParserError> {
        Ok(SixelEvent::GotoNextLine)
    }
    fn emit_possible_pending_event(&mut self, mut cb: impl FnMut(SixelEvent)) {
        if !self.currently_parsing.is_empty() || !self.pending_event_fields.is_empty() {
            match self.state {
                ParserState::ColorIntroducer => cb(self.emit_color_introducer_event().unwrap()), // TODO: not unwrap
                ParserState::RasterAttribute => cb(self.emit_raster_attribute_event().unwrap()), // TODO: not unwrap
                _ => {}
            };
        }
    }
    fn emit_single_byte_event(&mut self, byte: u8) -> Result<SixelEvent, ParserError> {
        match byte {
            b'?'..=b'~' => {
                self.emit_sixel_data_event(byte)
            }
            b'$' => {
                self.emit_beginning_of_line_event()
            }
            b'-' => {
                self.emit_next_line_event()
            }
            _ => {
                Err(ParserError::ParsingError)
            }
        }
    }
    fn finalize_field(&mut self) -> Result<(), ParserError> {
        if !self.currently_parsing.is_empty() {
            self.pending_event_fields
                .try_push(self.currently_parsing.drain(..).collect())?;
        }
        Ok(())
    }
    fn emit_event(&mut self, event: SixelEvent) {
        println!("emitting: {:?}", event);
        // (self.cb)(event);
        // self.events.push(event);
    }
    fn clear(&mut self) {}
}

#[cfg(test)]
#[path = "./tests.rs"]
mod tests;
