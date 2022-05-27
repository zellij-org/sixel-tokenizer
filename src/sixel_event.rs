use arrayvec::ArrayVec;
use std::str;

use crate::ParserError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SixelEvent {
    ColorIntroducer {
        color_number: u16,
        color_coordinate_system: Option<ColorCoordinateSystem>,
    },
    RasterAttribute {
        pan: usize,
        pad: usize,
        ph: Option<usize>,
        pv: Option<usize>,
    },
    Data {
        byte: u8,
    },
    Repeat {
        repeat_count: usize,
        byte_to_repeat: u8,
    },
    Dcs {
        macro_parameter: Option<u8>,
        inverse_background: Option<u8>,
        horizontal_pixel_distance: Option<usize>,
    },
    GotoBeginningOfLine,
    GotoNextLine,
    UnknownSequence([Option<u8>; 5]),
    End,
}

impl SixelEvent {
    pub fn new_dcs(
        macro_parameter: Option<u8>,
        inverse_background: Option<u8>,
        horizontal_pixel_distance: Option<usize>,
    ) -> SixelEvent {
        SixelEvent::Dcs {
            macro_parameter,
            inverse_background,
            horizontal_pixel_distance,
        }
    }
    pub fn new_color_introducer(
        color_number: u16,
        coordinate_system_indicator: Option<u8>,
        x: Option<usize>,
        y: Option<usize>,
        z: Option<usize>,
    ) -> Result<SixelEvent, ParserError> {
        match (coordinate_system_indicator, x, y, z) {
            (Some(coordinate_system_indicator), Some(x), Some(y), Some(z)) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: Some(ColorCoordinateSystem::new(
                        coordinate_system_indicator,
                        x,
                        y,
                        z,
                    )?),
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
            _ => Err(ParserError::ParsingError),
        }
    }
    pub fn color_introducer_from_fields(
        pending_event_fields: &mut ArrayVec<ArrayVec<u8, 5>, 5>,
    ) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let color_number = mandatory_field_u16(byte_fields.next())?;
        let coordinate_system_indicator = optional_field(byte_fields.next())?;
        let x = optional_usize_field(byte_fields.next())?;
        let y = optional_usize_field(byte_fields.next())?;
        let z = optional_usize_field(byte_fields.next())?;
        match (coordinate_system_indicator, x, y, z) {
            (Some(coordinate_system_indicator), Some(x), Some(y), Some(z)) => {
                let event = SixelEvent::ColorIntroducer {
                    color_number,
                    color_coordinate_system: Some(ColorCoordinateSystem::new(
                        coordinate_system_indicator,
                        x,
                        y,
                        z,
                    )?),
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
            _ => Err(ParserError::ParsingError),
        }
    }
    pub fn new_raster(
        pan: usize,
        pad: usize,
        ph: Option<usize>,
        pv: Option<usize>,
    ) -> Result<SixelEvent, ParserError> {
        let event = SixelEvent::RasterAttribute { pan, pad, ph, pv };
        Ok(event)
    }
    pub fn raster_attribute_from_fields(
        pending_event_fields: &mut ArrayVec<ArrayVec<u8, 5>, 5>,
    ) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let pan = mandatory_usize_field(byte_fields.next())?;
        let pad = mandatory_usize_field(byte_fields.next())?;
        let ph = optional_usize_field(byte_fields.next())?;
        let pv = optional_usize_field(byte_fields.next())?;
        if byte_fields.next().is_some() {
            return Err(ParserError::ParsingError);
        }
        let event = SixelEvent::RasterAttribute { pan, pad, ph, pv };
        Ok(event)
    }
    pub fn dcs_from_fields(
        pending_event_fields: &mut ArrayVec<ArrayVec<u8, 5>, 5>,
    ) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let macro_parameter = optional_field(byte_fields.next())?;
        let inverse_background = optional_field(byte_fields.next())?;
        let horizontal_pixel_distance = optional_usize_field(byte_fields.next())?;
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
    pub fn new_repeat(repeat_count: usize, byte_to_repeat: u8) -> Result<SixelEvent, ParserError> {
        let event = SixelEvent::Repeat {
            repeat_count,
            byte_to_repeat,
        };
        Ok(event)
    }
    pub fn repeat_from_fields(
        pending_event_fields: &mut ArrayVec<ArrayVec<u8, 5>, 5>,
        byte_to_repeat: u8,
    ) -> Result<SixelEvent, ParserError> {
        let mut byte_fields = pending_event_fields.drain(..);
        let repeat_count = mandatory_usize_field(byte_fields.next())?;
        if byte_fields.next().is_some() {
            return Err(ParserError::ParsingError);
        }
        let event = SixelEvent::Repeat {
            repeat_count: repeat_count.into(),
            byte_to_repeat,
        };
        Ok(event)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorCoordinateSystem {
    HLS(usize, usize, usize),
    RGB(usize, usize, usize),
}

impl ColorCoordinateSystem {
    pub fn new(
        coordinate_system_indicator: u8,
        x: usize,
        y: usize,
        z: usize,
    ) -> Result<Self, ParserError> {
        match coordinate_system_indicator {
            1 => Ok(ColorCoordinateSystem::HLS(x, y, z)),
            2 => Ok(ColorCoordinateSystem::RGB(x, y, z)),
            _ => Err(ParserError::ParsingError),
        }
    }
}

fn bytes_to_u8(bytes: ArrayVec<u8, 5>) -> Result<u8, ParserError> {
    Ok(u8::from_str_radix(str::from_utf8(&bytes)?, 10)?)
}

fn bytes_to_u16(bytes: ArrayVec<u8, 5>) -> Result<u16, ParserError> {
    Ok(u16::from_str_radix(str::from_utf8(&bytes)?, 10)?)
}

fn bytes_to_usize(bytes: ArrayVec<u8, 5>) -> Result<usize, ParserError> {
    Ok(usize::from_str_radix(str::from_utf8(&bytes)?, 10)?)
}

fn mandatory_field_u16(field: Option<ArrayVec<u8, 5>>) -> Result<u16, ParserError> {
    bytes_to_u16(field.ok_or(ParserError::ParsingError)?)
}

fn mandatory_usize_field(field: Option<ArrayVec<u8, 5>>) -> Result<usize, ParserError> {
    bytes_to_usize(field.ok_or(ParserError::ParsingError)?)
}

fn optional_field(field: Option<ArrayVec<u8, 5>>) -> Result<Option<u8>, ParserError> {
    match field {
        Some(field) => {
            let parsed = bytes_to_u8(field)?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

fn optional_usize_field(field: Option<ArrayVec<u8, 5>>) -> Result<Option<usize>, ParserError> {
    match field {
        Some(field) => {
            let parsed = bytes_to_usize(field)?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}
