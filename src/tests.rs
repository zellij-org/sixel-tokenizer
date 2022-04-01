use arrayvec::{ArrayVec, CapacityError};
use insta::assert_snapshot;
use std::num::ParseIntError;
use std::str::{self, Utf8Error};

use crate::Parser;

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
    // TBD
}

#[test]
fn dcs_event_with_all_optional_fields () {
    // TBD
}

#[test]
fn dcs_event_with_partial_optional_fields() {
    // TBD
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
