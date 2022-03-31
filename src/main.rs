use ::insta::assert_snapshot;
use std::str::{self, Utf8Error};
use std::num::ParseIntError;
use arrayvec::{ArrayVec, CapacityError};

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
    GraphicsRepeatIntroducer,
    UnknownInstruction,
}

#[derive(Clone, Copy, Debug)]
pub enum SixelEvent {
    ColorIntroducer {
        color_number: u8,
        color_coordinate_system: Option<ColorCoordinateSystem>,
    },
    Data {
        byte: u8
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

pub struct Parser <'a>{
    state: ParserState,
    raw_instruction: ArrayVec<u8, 256>, // TODO: proper cap
    intermediate_color_introducer: ArrayVec<ArrayVec<u8, 3>, 5>,
    intermediate_dcs: ArrayVec<ArrayVec<u8, 3>, 3>,
    intermediate_repeat: ArrayVec<ArrayVec<u8, 3>, 3>,
    currently_parsing: ArrayVec<u8, 256>, // TODO: proper cap
    events: &'a mut Vec<SixelEvent>,
}

impl <'a> Parser <'a>{
    pub fn new(events: &'a mut Vec<SixelEvent>) -> Self {
        Parser {
            state: ParserState::Ground,
            raw_instruction: ArrayVec::new(),
            intermediate_color_introducer: ArrayVec::new(),
            intermediate_dcs: ArrayVec::new(),
            intermediate_repeat: ArrayVec::new(),
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
        match self.state {
            ParserState::Ground => {
                self.handle_ground(*byte);
            }
            ParserState::EscapeCharacter => {
                if let Err(err) = self.handle_escape_character(*byte) {
                    self.state = ParserState::Ground;
                    self.handle_ground(*byte);
                }
            }
            ParserState::DeviceControlString => {
                if let Err(err) = self.handle_device_control_string(*byte) {
                    self.state = ParserState::Ground;
                    self.handle_ground(*byte);
                }
            }
            ParserState::ColorIntroducer => {
                if let Err(err) = self.handle_color_introducer(*byte) {
                    self.state = ParserState::Ground;
                    self.handle_ground(*byte);
                }
            }
            ParserState::GraphicsRepeatIntroducer => {
                if let Err(err) = self.handle_repeat_introducer(*byte) {
                    self.state = ParserState::Ground;
                    self.handle_ground(*byte);
                }
            }
            ParserState::UnknownInstruction => {

            }
            _ => {}
        }
    }
    fn handle_ground(&mut self, byte: u8) {
        match byte {
            b'?'..=b'~' => {
                self.emit_sixel_data(byte);
            }
            b'$' => {
                self.emit_beginning_of_line_event();
            }
            b'-' => {
                self.emit_next_line_event();
            }
            _ => {}
        }
        self.state = next_state(self.state, byte);
    }
    fn handle_device_control_string(&mut self, byte: u8) -> Result<(), ParserError> {
        if byte == b';' {
            self.finalize_dcs_field()?;
            Ok(())
        } else if byte == b'q' {
            self.finalize_dcs_field()?;
            self.emit_dcs_event()?;
            self.state = ParserState::Ground;
            Ok(())
        } else if let b'0'..=b'9' = byte {
            self.currently_parsing.push(byte);
            Ok(())
        } else {
            self.finalize_dcs_field()?;
            self.emit_dcs_event()?;
            Err(ParserError::ParsingError)
        }
    }
    fn handle_color_introducer(&mut self, byte: u8) -> Result<(), ParserError> {
        if byte == b';' {
            self.finalize_color_introducer_field()?;
            Ok(())
        } else if let b'0'..=b'9' = byte{
            self.currently_parsing.push(byte);
            Ok(())
        } else {
            self.finalize_color_introducer_field()?;
            self.emit_color_introducer_event()?;
            Err(ParserError::ParsingError)
        }
    }
    fn handle_escape_character(&mut self, byte: u8) -> Result<(), ParserError> {
        if byte == b'P' {
            self.state = ParserState::DeviceControlString;
            Ok(())
        } else if byte == b'\\' {
            // end sixel sequence
            self.state = ParserState::Ground;
            self.clear();
            Ok(())
        } else {
            Err(ParserError::ParsingError)
        }
    }
    fn handle_repeat_introducer(&mut self, byte: u8) -> Result<(), ParserError> {
        if let b'0'..=b'9' = byte {
            self.currently_parsing.push(byte);
            Ok(())
        } else if let b'?'..=b'~' = byte {
            self.finalize_repeat_introducer_field()?;
            self.emit_repeat_introducer_event(byte)?;
            self.state = ParserState::Ground;
            Ok(())
        } else {
            Err(ParserError::ParsingError)
        }
    }
    fn finalize_color_introducer_field(&mut self) -> Result<(), ParserError> {
        if !self.currently_parsing.is_empty() {
            let currently_parsing = self.currently_parsing.drain(..);
            if currently_parsing.len() > 3 {
                return Err(ParserError::ParsingError);
            } else {
                let currently_parsing: ArrayVec<u8, 3> = currently_parsing.collect();
                self.intermediate_color_introducer.try_push(
                    currently_parsing
                )?;
            }
        }
        Ok(())
    }
    fn finalize_dcs_field(&mut self) -> Result<(), ParserError> {
        if !self.currently_parsing.is_empty() {
            self.intermediate_dcs.try_push(self.currently_parsing.drain(..).collect())?;
        }
        Ok(())
    }
    fn finalize_repeat_introducer_field(&mut self) -> Result<(), ParserError> {
        if !self.currently_parsing.is_empty() {
            self.intermediate_repeat.try_push(self.currently_parsing.drain(..).collect())?;
        }
        Ok(())
    }
    fn emit_color_introducer_event(&mut self) -> Result<(), ParserError> {
        let mut byte_fields = self.intermediate_color_introducer.drain(..);
        let color_number = byte_fields.next().map(|f| bytes_to_u8(f)).ok_or(ParserError::ParsingError)?;
        let coordinate_system_indicator = byte_fields.next().map(|f| bytes_to_u8(f));
        let x = byte_fields.next().map(|f| bytes_to_u8(f));
        let y = byte_fields.next().map(|f| bytes_to_u8(f));
        let z = byte_fields.next().map(|f| bytes_to_u8(f));
        match (color_number, coordinate_system_indicator, x, y, z) {
            (Ok(color_number), Some(Ok(coordinate_system_indicator)), Some(Ok(x)), Some(Ok(y)), Some(Ok(z))) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: Some(ColorCoordinateSystem::new(coordinate_system_indicator, x, y, z).unwrap()), // TODO: handle err
                };
                self.events.push(event);
                Ok(())
            },
            (Ok(color_number), _, _, _, _) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: None
                };
                self.events.push(event);
                Ok(())
            }
            _ => {
                Err(ParserError::ParsingError)
            }
        }
    }
    fn emit_dcs_event(&mut self) -> Result<(), ParserError> {
        let mut byte_fields = self.intermediate_dcs.drain(..);
        let macro_parameter = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten();
        let inverse_background = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten();
        let horizontal_pixel_distance = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten();
        let event = SixelEvent::Dcs {
            macro_parameter,
            inverse_background,
            horizontal_pixel_distance,
        };
        self.events.push(event);
        Ok(())
    }
    fn emit_repeat_introducer_event(&mut self, byte_to_repeat: u8) -> Result<(), ParserError> {
        let mut byte_fields = self.intermediate_repeat.drain(..);
        let repeat_count = byte_fields.next().map(|f| bytes_to_u8(f).ok()).flatten().ok_or(ParserError::ParsingError)?;
        let event = SixelEvent::Repeat {
            repeat_count,
            byte_to_repeat
        };
        self.events.push(event);
        Ok(())
    }
    fn emit_sixel_data(&mut self, byte: u8) {
        let event = SixelEvent::Data {
            byte
        };
        self.events.push(event);
    }
    fn emit_beginning_of_line_event(&mut self) {
        let event = SixelEvent::GotoBeginningOfLine;
        self.events.push(event);
    }
    fn emit_next_line_event(&mut self) {
        let event = SixelEvent::GotoNextLine;
        self.events.push(event);
    }
    fn clear(&mut self) {

    }
}

fn next_state(current_state: ParserState, byte: u8) -> ParserState {
    match (current_state, byte) {
        (ParserState::EscapeCharacter, b'P') => ParserState::DeviceControlString,
        (ParserState::DeviceControlString, b'q') => ParserState::Ground,
        (_, b'#') => ParserState::ColorIntroducer,
        (_, b'!') => ParserState::GraphicsRepeatIntroducer,
        (_, b'$') => ParserState::Ground,
        (_, b'-') => ParserState::Ground,
        (_, 27) => ParserState::EscapeCharacter,
        _ => current_state
    }
}

fn bytes_to_u8(bytes: ArrayVec<u8, 3>) -> Result<u8, ParserError> {
    // TODO: error handling, return Result
    // let bytes: ArrayVec<u8, 3> = bytes.iter().map(|b| b + 48).collect(); // + 48 to assume it's a numerical digit
    Ok(u8::from_str_radix(str::from_utf8(&bytes)?, 10)?)
}

#[test]
fn basic_sample() {
    let sample = "
        \u{1b}Pq
        #0;2;0;0;0#1;2;100;100;0#2;2;0;100;0
        #1~~@@vv@@~~@@~~$
        #2??}}GG}}??}}??-
        #1!14@
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new(&mut events);
    for byte in sample_bytes {
        parser.advance(byte);
    };
    let mut snapshot = String::new();
    for event in events {
        snapshot.push_str(&format!("{:?}", event));
        snapshot.push('\n');
    }

    assert_snapshot!(snapshot);
}
