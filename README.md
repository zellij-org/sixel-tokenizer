# sixel-tokenizer

This is tokenizer for serialized sixel bytes.

It turns this:
```
<ESC>Pq
\"2;1;100;200
#0;2;0;0;0#1;2;100;100;0#2;2;0;100;0
#1~~@@vv@@~~@@~~$
#2??}}GG}}??}}??-
#1!14@
<ESC>\\
```

Into this:
```
Dcs { macro_parameter: None, inverse_background: None, horizontal_pixel_distance: None }
RasterAttribute { pan: 2, pad: 1, ph: Some(100), pv: Some(200) }
ColorIntroducer { color_number: 0, color_coordinate_system: Some(RGB(0, 0, 0)) }
ColorIntroducer { color_number: 1, color_coordinate_system: Some(RGB(100, 100, 0)) }
ColorIntroducer { color_number: 2, color_coordinate_system: Some(RGB(0, 100, 0)) }
ColorIntroducer { color_number: 1, color_coordinate_system: None }
Data { byte: 126 }
...
GotoNextLine
ColorIntroducer { color_number: 1, color_coordinate_system: None }
Repeat { repeat_count: 14, byte_to_repeat: 64 }
End
```

## Example

```rust
use sixel_tokenizer::Parser;

fn main() {
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
    println!("{}", snapshot);
}
```
