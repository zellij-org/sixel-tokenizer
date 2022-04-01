use arrayvec::{ArrayVec, CapacityError};
use std::num::ParseIntError;
use std::str::{self, Utf8Error};

use thiserror::Error;

use crate::ParserError;

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

impl SixelEvent {
    pub fn new_color_introducer(pending_event_fields: &mut ArrayVec<ArrayVec<u8, 3>, 5>) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let color_number = mandatory_field(byte_fields.next())?;
        let coordinate_system_indicator = optional_field(byte_fields.next())?;
        let x = optional_field(byte_fields.next())?;
        let y = optional_field(byte_fields.next())?;
        let z = optional_field(byte_fields.next())?;
        match (coordinate_system_indicator, x, y, z) {
            (
                Some(coordinate_system_indicator),
                Some(x),
                Some(y),
                Some(z),
            ) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: Some(
                        ColorCoordinateSystem::new(coordinate_system_indicator, x, y, z).unwrap(),
                    ), // TODO: handle err
                };
                Ok(event)
            }
            (None, None, None, None) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: None,
                };
                Ok(event)
            }
            _ => {
                Err(ParserError::ParsingError)
            }
        }
    }
    pub fn new_raster_attribute(pending_event_fields: &mut ArrayVec<ArrayVec<u8, 3>, 5>) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let pan = mandatory_field(byte_fields.next())?;
        let pad = mandatory_field(byte_fields.next())?;
        let ph = optional_field(byte_fields.next())?;
        let pv = optional_field(byte_fields.next())?;
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
    pub fn new_dcs(pending_event_fields: &mut ArrayVec<ArrayVec<u8, 3>, 5>) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let macro_parameter = optional_field(byte_fields.next())?;
        let inverse_background = optional_field(byte_fields.next())?;
        let horizontal_pixel_distance = optional_field(byte_fields.next())?;
        if byte_fields.next().is_some() {
            return Err(ParserError::ParsingError);
        }
        let event = SixelEvent::Dcs {
            macro_parameter,
            inverse_background,
            horizontal_pixel_distance,
        };
        Ok(event)
    }
    pub fn new_repeat(pending_event_fields: &mut ArrayVec<ArrayVec<u8, 3>, 5>, byte_to_repeat: u8) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let repeat_count = mandatory_field(byte_fields.next())?;
        if byte_fields.next().is_some() {
            return Err(ParserError::ParsingError);
        }
        let event = SixelEvent::Repeat {
            repeat_count,
            byte_to_repeat,
        };
        Ok(event)
    }
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

fn bytes_to_u8(bytes: ArrayVec<u8, 3>) -> Result<u8, ParserError> {
    // TODO: error handling, return Result
    Ok(u8::from_str_radix(str::from_utf8(&bytes)?, 10)?)
}

fn mandatory_field (field: Option<ArrayVec<u8, 3>>) -> Result<u8, ParserError> {
    bytes_to_u8(field.ok_or(ParserError::ParsingError)?)
}

fn optional_field(field: Option<ArrayVec<u8, 3>>) -> Result<Option<u8>, ParserError> {
    match field {
        Some(field) => {
            let parsed = bytes_to_u8(field)?;
            Ok(Some(parsed))
        }
        None => Ok(None)
    }
}
