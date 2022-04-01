use arrayvec::{ArrayVec, CapacityError};
use std::num::ParseIntError;
use std::str::{self, Utf8Error};

use thiserror::Error;

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
    let mut parser = Parser::new(&mut events);
    for byte in sample_bytes {
        parser.advance(byte);
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

#[derive(Clone, Copy, Debug)]
pub enum SixelEvent {
    ColorIntroducer {
        color_number: u8,
        color_coordinate_system: Option<ColorCoordinateSystem>,
    },
    RasterAttribute {
        pan: u8,
        pad: u8,
        ph: Option<u8>,
        pv: Option<u8>,
    },
    Data {
        byte: u8,
    },
    Repeat {
        repeat_count: u8, // TODO: size?
        byte_to_repeat: u8,
    },
    Dcs {
        macro_parameter: Option<u8>,
        inverse_background: Option<u8>,
        horizontal_pixel_distance: Option<u8>,
    },
    GotoBeginningOfLine,
    GotoNextLine,
}

#[derive(Clone, Copy, Debug)]
pub enum ColorCoordinateSystem {
    HLS(u8, u8, u8),
    RGB(u8, u8, u8),
}

impl ColorCoordinateSystem {
    pub fn new(coordinate_system_indicator: u8, x: u8, y: u8, z: u8) -> Result<Self, &'static str> {
        match coordinate_system_indicator {
            1 => Ok(ColorCoordinateSystem::HLS(x, y, z)),
            2 => Ok(ColorCoordinateSystem::RGB(x, y, z)),
            _ => Err("coordinate_system_indicator must be 1 or 2"), // TODO: say what we got
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ParsingResult {
    EmitEvent(SixelEvent),
    WaitingForMore,
}

pub struct Parser<'a> {
    state: ParserState,
    raw_instruction: ArrayVec<u8, 256>, // TODO: proper cap
    pending_event_fields: ArrayVec<ArrayVec<u8, 3>, 5>,
    currently_parsing: ArrayVec<u8, 256>, // TODO: proper cap
    events: &'a mut Vec<SixelEvent>,
}

impl<'a> Parser<'a> {
    pub fn new(events: &'a mut Vec<SixelEvent>) -> Self {
        Parser {
            state: ParserState::Ground,
            raw_instruction: ArrayVec::new(),
            pending_event_fields: ArrayVec::new(),
            currently_parsing: ArrayVec::new(),
            events,
        }
    }
    pub fn advance(&mut self, byte: &u8) {
        if byte == &b' ' || byte == &b'\n' || byte == &b'\t' {
            // ignore whitespace
            return;
        }
        self.raw_instruction.push(*byte);
        println!(
            "byte: {:?}, current_state: {:?}",
            str::from_utf8(&[*byte]),
            self.state
        );
        self.parse_byte(byte);
    }
    fn parse_byte(&mut self, byte: &u8) {
        self.move_to_next_state(*byte);
        if *byte == b';' {
            self.finalize_field().unwrap(); // TODO: not unwrap
        } else if let b'0'..=b'9' = byte {
            self.currently_parsing.push(*byte);
        }
    }
    fn emit_dcs_event(&mut self) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = self.create_dcs_event()?;
        self.emit_event(event);
        Ok(())
        // TODO: clear raw
    }
    fn emit_color_introducer_event(&mut self) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = self.create_color_introducer_event()?;
        self.emit_event(event);
        Ok(())
        // TODO: clear raw
    }
    fn emit_raster_attribute_event(&mut self) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = self.create_raster_attribute_event()?;
        self.emit_event(event);
        Ok(())
        // TODO: clear raw
    }
    fn emit_sixel_data_event(&mut self, byte: u8) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = self.create_sixel_data_event(byte);
        self.emit_event(event);
        Ok(())
    }
    fn emit_repeat_introducer_event(&mut self, byte: u8) -> Result<(), ParserError> {
        self.finalize_field()?;
        let event = self.create_repeat_introducer_event(byte)?;
        self.emit_event(event);
        Ok(())
    }
    fn emit_beginning_of_line_event(&mut self) -> Result<(), ParserError> {
        let event = self.create_beginning_of_line_event();
        self.emit_event(event);
        Ok(())
    }
    fn emit_next_line_event(&mut self) -> Result<(), ParserError> {
        let event = self.create_next_line_event();
        self.emit_event(event);
        Ok(())
    }
    fn emit_possible_pending_event(&mut self) -> Result<(), ParserError> {
        if self.currently_parsing.is_empty() && self.pending_event_fields.is_empty() {
            Ok(())
        } else {
            match self.state {
                ParserState::ColorIntroducer => self.emit_color_introducer_event()?,
                ParserState::RasterAttribute => self.emit_raster_attribute_event()?,
                _ => {}
            }
            Ok(())
        }
    }
    fn emit_single_byte_event(&mut self, byte: u8) -> Result<(), ParserError> {
        match byte {
            b'?'..=b'~' => {
                self.emit_sixel_data_event(byte)?
            }
            b'$' => {
                self.emit_beginning_of_line_event()?
            }
            b'-' => {
                self.emit_next_line_event()?
            }
            _ => {}
        }
        Ok(())
    }
    fn move_to_next_state(&mut self, byte: u8) {
        let current_state = self.state;
        match (current_state, byte) {
            (ParserState::EscapeCharacter, b'P' | b'\\') => {
                self.state = ParserState::DeviceControlString;
            }
            (ParserState::DeviceControlString, b'q') => {
                self.emit_dcs_event().unwrap(); // TODO: not unwrap
                self.state = ParserState::Ground;
            }
            (ParserState::GraphicsRepeatIntroducer, b'?'..=b'~') => {
                self.emit_repeat_introducer_event(byte).unwrap(); // TODO: not unwrap
            }
            (_, b'?'..=b'~' | b'$' | b'-') => {
                self.emit_possible_pending_event().unwrap(); // TODO: not unwrap
                self.emit_single_byte_event(byte).unwrap(); // TODO: not unwrap
                self.state = ParserState::Ground;
            }
            (_, b'#') => {
                self.emit_possible_pending_event().unwrap(); // TODO: not unwrap
                self.state = ParserState::ColorIntroducer;
            }
            (_, b'"') => {
                self.emit_possible_pending_event().unwrap(); // TODO: not unwrap
                self.state = ParserState::RasterAttribute;
            }
            (_, b'!') => {
                self.emit_possible_pending_event().unwrap(); // TODO: not unwrap
                self.state = ParserState::GraphicsRepeatIntroducer;
            }
            (_, 27) => {
                self.emit_possible_pending_event().unwrap(); // TODO: not unwrap
                self.state = ParserState::EscapeCharacter;
            }
            _ => {}
        };
    }
    fn finalize_field(&mut self) -> Result<(), ParserError> {
        if !self.currently_parsing.is_empty() {
            self.pending_event_fields
                .try_push(self.currently_parsing.drain(..).collect())?;
        }
        Ok(())
    }
    fn create_color_introducer_event(&mut self) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = self.pending_event_fields.drain(..);
        let color_number = byte_fields
            .next()
            .map(|f| bytes_to_u8(f))
            .ok_or(ParserError::ParsingError)?;
        let coordinate_system_indicator = byte_fields.next().map(|f| bytes_to_u8(f));
        let x = byte_fields.next().map(|f| bytes_to_u8(f));
        let y = byte_fields.next().map(|f| bytes_to_u8(f));
        let z = byte_fields.next().map(|f| bytes_to_u8(f));
        match (color_number, coordinate_system_indicator, x, y, z) {
            (
                Ok(color_number),
                Some(Ok(coordinate_system_indicator)),
                Some(Ok(x)),
                Some(Ok(y)),
                Some(Ok(z)),
            ) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: Some(
                        ColorCoordinateSystem::new(coordinate_system_indicator, x, y, z).unwrap(),
                    ), // TODO: handle err
                };
                Ok(event)
            }
            (Ok(color_number), _, _, _, _) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: None,
                };
                Ok(event)
            }
            _ => Err(ParserError::ParsingError),
        }
    }
    fn create_raster_attribute_event(&mut self) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = self.pending_event_fields.drain(..);
        let pan = bytes_to_u8(byte_fields.next().ok_or(ParserError::ParsingError)?)?;
        let pad = bytes_to_u8(byte_fields.next().ok_or(ParserError::ParsingError)?)?;
        let ph = byte_fields.next().and_then(|f| bytes_to_u8(f).ok());
        let pv = byte_fields.next().and_then(|f| bytes_to_u8(f).ok());
        if byte_fields.next().is_some() {
            return Err(ParserError::ParsingError);
        }
        let event = SixelEvent::RasterAttribute {
            pan,
            pad,
            ph,
            pv
        };
        Ok(event)
    }
    fn create_dcs_event(&mut self) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = self.pending_event_fields.drain(..);
        let macro_parameter = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten();
        let inverse_background = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten();
        let horizontal_pixel_distance = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten();
        let event = SixelEvent::Dcs {
            macro_parameter,
            inverse_background,
            horizontal_pixel_distance,
        };
        Ok(event)
    }
    fn create_repeat_introducer_event(
        &mut self,
        byte_to_repeat: u8,
    ) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = self.pending_event_fields.drain(..);
        let repeat_count = byte_fields
            .next()
            .map(|f| bytes_to_u8(f).ok())
            .flatten()
            .ok_or(ParserError::ParsingError)?;
        let event = SixelEvent::Repeat {
            repeat_count,
            byte_to_repeat,
        };
        Ok(event)
    }
    fn create_sixel_data_event(&mut self, byte: u8) -> SixelEvent {
        SixelEvent::Data { byte }
    }
    fn create_beginning_of_line_event(&mut self) -> SixelEvent {
        SixelEvent::GotoBeginningOfLine
    }
    fn create_next_line_event(&mut self) -> SixelEvent {
        SixelEvent::GotoNextLine
    }
    fn emit_event(&mut self, event: SixelEvent) {
        println!("emitting: {:?}", event);
        self.events.push(event);
    }
    fn clear(&mut self) {}
}

fn bytes_to_u8(bytes: ArrayVec<u8, 3>) -> Result<u8, ParserError> {
    // TODO: error handling, return Result
    Ok(u8::from_str_radix(str::from_utf8(&bytes)?, 10)?)
}

#[cfg(test)]
#[path = "./tests.rs"]
mod tests;
