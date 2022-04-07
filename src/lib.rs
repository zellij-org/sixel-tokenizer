use std::num::ParseIntError;
use std::str::Utf8Error;

use arrayvec::{ArrayVec, CapacityError};
use thiserror::Error;

mod sixel_event;
use sixel_event::SixelEvent;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Failed to parse")]
    ParsingError,
    #[error("Failed to parse")]
    CapacityError(#[from] CapacityError<ArrayVec<u8, 5>>),
    #[error("Failed to parse")]
    CapacityErrorU8(#[from] CapacityError<u8>),
    #[error("Failed to parse")]
    Utf8Error(#[from] Utf8Error),
    #[error("Failed to parse")]
    ParseIntError(#[from] ParseIntError),
}

#[derive(Clone, Copy, Debug)]
pub enum ParserState {
    Ground,
    DeviceControlString,
    EscapeCharacter,
    ColorIntroducer,
    RasterAttribute,
    GraphicsRepeatIntroducer,
    UnknownSequence,
}

pub struct Parser {
    state: ParserState,
    raw_instruction: ArrayVec<u8, 256>,
    pending_event_fields: ArrayVec<ArrayVec<u8, 5>, 5>,
    currently_parsing: ArrayVec<u8, 256>,
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
    pub fn advance(&mut self, byte: &u8, mut cb: impl FnMut(SixelEvent)) {
        if byte == &b' ' || byte == &b'\n' || byte == &b'\t' {
            // ignore whitespace
            return;
        }
        if let Err(e) = self.process_byte(*byte, &mut cb) {
            self.handle_error(e, Some(*byte), &mut cb);
        }
    }
    fn process_byte(
        &mut self,
        byte: u8,
        mut cb: impl FnMut(SixelEvent),
    ) -> Result<(), ParserError> {
        match (self.state, byte) {
            (ParserState::EscapeCharacter, b'P') => self.raw_instruction.try_push(byte)?,
            (ParserState::EscapeCharacter, b'\\') => self.emit_end_sequence(&mut cb)?,
            (ParserState::DeviceControlString, b'q') => self.emit_dcs_event(&mut cb)?,
            (ParserState::GraphicsRepeatIntroducer, b'?'..=b'~') => {
                self.emit_repeat_introducer_event(byte, &mut cb)?
            }
            (_, b'?'..=b'~' | b'$' | b'-') => {
                self.emit_possible_pending_event(&mut cb);
                self.emit_single_byte_event(byte, &mut cb)?;
            }
            (_, b';') => {
                self.raw_instruction.try_push(byte)?;
                self.finalize_field()?;
            }
            (_, b'0'..=b'9') => {
                self.raw_instruction.try_push(byte)?;
                self.currently_parsing.try_push(byte)?;
            }
            _ => {
                self.emit_possible_pending_event(&mut cb);
                self.raw_instruction.try_push(byte)?;
            }
        };
        self.move_to_next_state(byte);
        Ok(())
    }
    fn move_to_next_state(&mut self, byte: u8) {
        self.state = match (self.state, byte) {
            (ParserState::EscapeCharacter, b'P') => ParserState::DeviceControlString,
            (ParserState::EscapeCharacter, b'\\')
            | (ParserState::DeviceControlString, b'q')
            | (ParserState::GraphicsRepeatIntroducer, b'?'..=b'~') => ParserState::Ground,
            (_, b'?'..=b'~' | b'$' | b'-') => ParserState::Ground,
            (_, b'#') => ParserState::ColorIntroducer,
            (_, b'"') => ParserState::RasterAttribute,
            (_, b'!') => ParserState::GraphicsRepeatIntroducer,
            (_, b';' | b'0'..=b'9') => self.state,
            (_, 27) => ParserState::EscapeCharacter,
            _ => ParserState::UnknownSequence,
        };
    }
    fn handle_error(&mut self, err: ParserError, byte: Option<u8>, cb: impl FnMut(SixelEvent)) {
        match err {
            _ => {
                self.state = ParserState::UnknownSequence;
                self.pending_event_fields.clear();
                self.currently_parsing.clear();
                self.emit_unknown_sequences(cb, byte);
            }
        }
    }
    fn emit_dcs_event(&mut self, mut cb: impl FnMut(SixelEvent)) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = SixelEvent::dcs_from_fields(&mut self.pending_event_fields)?;
        self.raw_instruction.clear();
        cb(event);
        Ok(())
    }
    fn emit_end_sequence(&mut self, mut cb: impl FnMut(SixelEvent)) -> Result<(), ParserError> {
        self.finalize_field()?;
        self.clear();
        cb(SixelEvent::End);
        Ok(())
    }
    fn emit_repeat_introducer_event(
        &mut self,
        byte: u8,
        mut cb: impl FnMut(SixelEvent),
    ) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = SixelEvent::repeat_from_fields(&mut self.pending_event_fields, byte)?;
        self.raw_instruction.clear();
        cb(event);
        Ok(())
    }
    fn emit_possible_pending_event(&mut self, mut cb: impl FnMut(SixelEvent)) {
        match self.possible_pending_event() {
            Ok(Some(event)) => cb(event),
            Ok(None) => {}
            Err(e) => self.handle_error(e, None, &mut cb),
        }
    }
    fn emit_single_byte_event(
        &mut self,
        byte: u8,
        mut cb: impl FnMut(SixelEvent),
    ) -> Result<(), ParserError> {
        let event = match byte {
            b'?'..=b'~' => self.sixel_data_event(byte),
            b'$' => self.beginning_of_line_event(),
            b'-' => self.next_line_event(),
            _ => Err(ParserError::ParsingError),
        };
        cb(event?);
        Ok(())
    }
    fn emit_unknown_sequences(&mut self, mut cb: impl FnMut(SixelEvent), last_byte: Option<u8>) {
        loop {
            let mut bytes: [Option<u8>; 5] = Default::default();
            let unknown_sequence_elements = if self.raw_instruction.len() >= 5 {
                self.raw_instruction.drain(..5).chain(None)
            } else {
                self.raw_instruction.drain(..).chain(last_byte)
            };
            for (i, byte) in unknown_sequence_elements.enumerate() {
                bytes[i] = Some(byte);
            }
            cb(SixelEvent::UnknownSequence(bytes));
            if self.raw_instruction.is_empty() {
                break;
            }
        }
    }
    fn color_introducer_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        let event = SixelEvent::color_introducer_from_fields(&mut self.pending_event_fields)?;
        self.raw_instruction.clear();
        Ok(event)
    }
    fn raster_attribute_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        let event = SixelEvent::raster_attribute_from_fields(&mut self.pending_event_fields)?;
        self.raw_instruction.clear();
        Ok(event)
    }
    fn sixel_data_event(&mut self, byte: u8) -> Result<SixelEvent, ParserError> {
        self.finalize_field()?;
        self.raw_instruction.clear();
        Ok(SixelEvent::Data { byte })
    }
    fn beginning_of_line_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.raw_instruction.clear();
        Ok(SixelEvent::GotoBeginningOfLine)
    }
    fn next_line_event(&mut self) -> Result<SixelEvent, ParserError> {
        self.raw_instruction.clear();
        Ok(SixelEvent::GotoNextLine)
    }
    fn possible_pending_event(&mut self) -> Result<Option<SixelEvent>, ParserError> {
        let has_pending_event = !self.currently_parsing.is_empty()
            || !self.pending_event_fields.is_empty()
            || !self.raw_instruction.is_empty();
        if has_pending_event {
            match self.state {
                ParserState::ColorIntroducer => {
                    let event = self.color_introducer_event()?;
                    Ok(Some(event))
                }
                ParserState::RasterAttribute => {
                    let event = self.raster_attribute_event()?;
                    Ok(Some(event))
                }
                _ => Err(ParserError::ParsingError),
            }
        } else {
            Ok(None)
        }
    }
    fn finalize_field(&mut self) -> Result<(), ParserError> {
        if !self.currently_parsing.is_empty() {
            let mut field: ArrayVec<u8, 5> = Default::default();
            for byte in self.currently_parsing.drain(..) {
                // we don't use collect here because ArrayVec doesn't implement Try and so
                // we wouldn't be able to propagate errors
                field.try_push(byte)?;
            }
            self.pending_event_fields.try_push(field)?;
        }
        Ok(())
    }
    fn clear(&mut self) {
        drop(std::mem::replace(self, Parser::new()));
    }
}

#[cfg(test)]
#[path = "./tests.rs"]
mod tests;
