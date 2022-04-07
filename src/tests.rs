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
    let sample = "\u{1b}P2q";
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
    let sample = "\u{1b}P1122q\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::UnknownSequence([Some(27), Some(b'P'), Some(b'1'), Some(b'1'), Some(b'2')]),
        SixelEvent::UnknownSequence([Some(b'2'), Some(b'q'), None, None, None]),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn corrupted_partial_dcs_event() {
    let sample = "\u{1b}P%q\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::UnknownSequence([Some(27), Some(b'P'), None, None, None]),
        SixelEvent::UnknownSequence([Some(b'%'), None, None, None, None]),
        SixelEvent::Data { byte: b'q'},
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event () {
    let sample = "#2\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(2, None, None, None, None).unwrap(),
        SixelEvent::End
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_with_all_optional_fields () {
    let sample = "#0;1;100;150;200\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(0, Some(1), Some(100), Some(150), Some(200)).unwrap(),
        SixelEvent::End
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_with_partial_optional_fields() {
    let sample = "#0;1;100\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::UnknownSequence([Some(b'#'), Some(b'0'), Some(b';'), Some(b'1'), Some(b';')]),
        SixelEvent::UnknownSequence([Some(b'1'), Some(b'0'), Some(b'0'), None, None]),
        SixelEvent::End
    ];
    assert_eq!(events, expected);
}

#[test]
fn corrupted_color_introducer_event() {
    let sample = "#0;1!;100\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::UnknownSequence([Some(b'#'), Some(b'0'), Some(b';'), Some(b'1'), None]),
        SixelEvent::UnknownSequence([Some(b'!'), Some(b';'), Some(b'1'), Some(b'0'), Some(b'0')]),
        SixelEvent::End
    ];
    assert_eq!(events, expected);
}

#[test]
fn two_consecutive_color_introducer_events() {
    let sample = "#0;2;0;0;0#1;2;100;100;0#2;2;0;100;0";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::new_color_introducer(1, Some(2), Some(100), Some(100), Some(0)).unwrap(),
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_after_dcs() {
    let sample = "\u{1b}Pq#0;2;0;0;0\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_after_raster_event() {
    let sample = "\u{1b}Pq\"2;1;100;200#0;2;0;0;0\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_after_sixel_data_event() {
    let sample = "~#0;2;0;0;0\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'~' },
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_after_repeat_event() {
    let sample = "!14~#0;2;0;0;0\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_repeat(14, b'~').unwrap(),
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_after_eol_event() {
    let sample = "-#0;2;0;0;0\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn color_introducer_event_after_beginning_of_line_event() {
    let sample = "$#0;2;0;0;0\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::new_color_introducer(0, Some(2), Some(0), Some(0), Some(0)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event () {
    let sample = "\"2;1\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(2, 1, None,None).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_with_all_optional_fields () {
    let sample = "\"2;1;100;200\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_with_partial_optional_fields() {
    let sample = "\"2;1;100\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(2, 1, Some(100), None).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn corrupted_raster_event() {
    let sample = "\"2ff\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::UnknownSequence([Some(b'\"'), Some(b'2'), None, None, None]),
        SixelEvent::Data { byte: b'f' },
        SixelEvent::Data { byte: b'f' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn two_consecutive_raster_events() {
    let sample = "\"2;1\"1;2;100;100\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(2, 1, None, None).unwrap(),
        SixelEvent::new_raster(1, 2, Some(100), Some(100)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_after_dcs() {
    let sample = "
        \u{1b}Pq
        \"2;1;100;200
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_after_color_introducer_event() {
    let sample = "
        #1
        \"2;1;100;200
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(1, None, None, None, None).unwrap(),
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_after_sixel_data_event() {
    let sample = "
        ~
        \"2;1;100;200
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'~' },
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_after_repeat_event() {
    let sample = "
        !15@
        \"2;1;100;200
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_repeat(15, b'@').unwrap(),
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_after_eol_event() {
    let sample = "
        -
        \"2;1;100;200
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn raster_event_after_beginning_of_line_event() {
    let sample = "
        $
        \"2;1;100;200
        \u{1b}\\
    ";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event () {
    let sample = "@\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'@' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn two_consecutive_data_events() {
    let sample = "@f\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'@' },
        SixelEvent::Data { byte: b'f' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event_after_dcs() {
    let sample = "\u{1b}Pq?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event_after_color_introducer_event() {
    let sample = "#2?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(2, None, None, None, None).unwrap(),
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event_after_raster_event() {
    let sample = "\"2;1;100;200?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(2, 1, Some(100), Some(200)).unwrap(),
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event_after_repeat_event() {
    let sample = "!15??\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_repeat(15, b'?').unwrap(),
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event_after_eol_event() {
    let sample = "-?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn data_event_after_beginning_of_line_event() {
    let sample = "$?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event () {
    let sample = "!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn long_repeat_event () {
    let sample = "!298@\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_repeat(298, b'@').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn corrupted_repeat_event() {
    let sample = "!f?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::UnknownSequence([Some(b'!'), Some(b'f'), None, None, None]),
        SixelEvent::Data { byte: b'?' },
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn two_consecutive_repeat_events() {
    let sample = "!5?!14f\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::new_repeat(14, b'f').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event_after_dcs() {
    let sample = "\u{1b}Pq!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event_after_raster_event() {
    let sample = "\"1;1;1;1!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(1, 1, Some(1), Some(1)).unwrap(),
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event_after_sixel_data_event() {
    let sample = "@!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'@' },
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event_after_color_introducer_event() {
    let sample = "#0;1;2;100;2!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(0, Some(1), Some(2), Some(100), Some(2)).unwrap(),
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event_after_eol_event() {
    let sample = "-!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn repeat_event_after_beginning_of_line_event() {
    let sample = "$!5?\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::new_repeat(5, b'?').unwrap(),
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn next_line_event () {
    let sample = "-\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn two_consecutive_next_line_events() {
    let sample = "--\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn next_line_event_after_dcs() {
    let sample = "\u{1b}Pq-\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn next_line_event_after_raster_event() {
    let sample = "\"1;1;1;1-\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(1, 1, Some(1), Some(1)).unwrap(),
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn next_line_event_after_sixel_data_event() {
    let sample = "?-\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'?' },
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn next_line_event_after_color_introducer_event() {
    let sample = "#2-\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(2, None, None, None, None).unwrap(),
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn next_line_event_after_beginning_of_line_event() {
    let sample = "$-\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::GotoNextLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn beginning_of_line_event () {
    let sample = "$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn two_consecutive_beginning_of_line_events() {
    let sample = "$$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn beginning_of_line_event_after_dcs() {
    let sample = "\u{1b}Pq$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_dcs(None, None, None),
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn beginning_of_line_event_after_raster_event() {
    let sample = "\"1;1;1;1$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_raster(1, 1, Some(1), Some(1)).unwrap(),
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn beginning_of_line_event_after_sixel_data_event() {
    let sample = "?$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::Data { byte: b'?' },
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn beginning_of_line_event_after_color_introducer_event() {
    let sample = "#1$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::new_color_introducer(1, None, None, None, None).unwrap(),
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

#[test]
fn beginning_of_line_event_after_next_line_event() {
    let sample = "-$\u{1b}\\";
    let sample_bytes = sample.as_bytes();
    let mut events = vec![];
    let mut parser = Parser::new();
    for byte in sample_bytes {
        parser.advance(&byte, |sixel_event| events.push(sixel_event));
    }
    let expected = vec![
        SixelEvent::GotoNextLine,
        SixelEvent::GotoBeginningOfLine,
        SixelEvent::End,
    ];
    assert_eq!(events, expected);
}

// TODO: same tests for gotobeginningofline events and test unknown sequences longer than 5
