extern crate tui;

use std::iter::repeat;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::Widget;

pub fn draw_card(val: u8, count: usize) -> String {
    if count != 0 {
        format!(
            "╒═╕{topright}\n\
             │{val}│{mid}\n\
             ╘═╛{botright}",
            val = val,
            topright = repeat('╕').take(count - 1).collect::<String>(),
            mid = repeat('│').take(count - 1).collect::<String>(),
            botright = repeat('╛').take(count - 1).collect::<String>()
        )
    } else {
        String::from("\n\n\n")
    }
}

pub struct PlaceWidget<'a>(pub &'a crate::mechanics::Place);

impl<'a> Widget for PlaceWidget<'a> {
    // A place is built around 7x7 squares on top of each other, forming a 7x21 area.
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        // TODO: consider being more efficient.
        let top_square = draw_card(self.0.value, self.0.scores[0] as usize);
        let main_square = format!(
            "\
             ╒═════╕\n\
             │{}    │\n\
             │     │\n\
             │    {}│\n\
             ╘═════╛",
            self.0.value, self.0.value
        );
        let bottom_square = draw_card(self.0.value, self.0.scores[1] as usize);
        for (count, line) in top_square
            .lines()
            .chain(main_square.lines())
            .chain(bottom_square.lines())
            .enumerate()
        {
            let count = count as u16;
            buf.set_string(area.left(), area.top() + count, line, Style::default());
        }
    }
}
