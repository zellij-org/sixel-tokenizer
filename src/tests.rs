use arrayvec::{ArrayVec, CapacityError};
use insta::assert_snapshot;
use std::num::ParseIntError;
use std::str::{self, Utf8Error};

use super::Parser;

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
    }
    let mut snapshot = String::new();
    for event in events {
        snapshot.push_str(&format!("{:?}", event));
        snapshot.push('\n');
    }

    assert_snapshot!(snapshot);
}

#[test]
fn sample_with_raster_attributes() {
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
    let mut parser = Parser::new(&mut events);
    for byte in sample_bytes {
        parser.advance(byte);
    }
    let mut snapshot = String::new();
    for event in events {
        snapshot.push_str(&format!("{:?}", event));
        snapshot.push('\n');
    }

    assert_snapshot!(snapshot);
}
