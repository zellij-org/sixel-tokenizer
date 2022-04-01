use arrayvec::{ArrayVec, CapacityError};
use insta::assert_snapshot;
use std::num::ParseIntError;
use std::str::{self, Utf8Error};

use crate::{Parser, SixelEvent};

#[test]
fn basic_sample() {
    let sample = "
        \u{1b}Pq
        \"2;1;100;200
        #0;2;0;0;0#1;2;100;100;0#2;2;0;100;0
        #1~~@@vv@@~~@@~~$
        #2??}}GG}}??}}??-
        #1!14@
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let mut snapshot = String::new();
    for event in events {
        snapshot.push_str(&format!("{:?}", event));
        snapshot.push('\n');
    }

    assert_snapshot!(snapshot);
}

#[test]
fn dcs_event () {
    let sample = "\u{1b}Pq";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None)
    ];
    assert_eq!(events, expected);
}

#[test]
fn dcs_event_with_all_optional_fields () {
    let sample = "\u{1b}P2;1;005;q"; // the 00 padding is added just to make sure we can handle it
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(Some(2), Some(1), Some(5))
    ];
    assert_eq!(events, expected);
}

#[test]
fn dcs_event_with_partial_optional_fields() {
    let sample = "\u{1b}P2q"; // the 00 padding is added just to make sure we can handle it
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(Some(2), None, None)
    ];
    assert_eq!(events, expected);
}

#[test]
fn corrupted_dcs_event() {
    // TBD
    // TODO: various corrupted types
}

#[test]
fn color_introducer_event () {
    // TBD
}

#[test]
fn color_introducer_event_with_all_optional_fields () {
    // TBD
}

#[test]
fn color_introducer_event_with_partial_optional_fields() {
    // TBD
}

#[test]
fn corrupted_color_introducer_event() {
    // TBD
    // TODO: various corrupted types
}

#[test]
fn two_consecutive_color_introducer_events() {
    // TBD
}

#[test]
fn color_introducer_event_after_dcs() {
    // TBD
}

#[test]
fn color_introducer_event_after_raster_event() {
    // TBD
}

#[test]
fn color_introducer_event_after_sixel_data_event() {
    // TBD
}

#[test]
fn color_introducer_event_after_repeat_event() {
    // TBD
}

#[test]
fn color_introducer_event_after_eol_event() {
    // TBD
}

#[test]
fn color_introducer_event_after_newline_event() {
    // TBD
}
