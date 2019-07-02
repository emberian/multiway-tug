extern crate tui;

use std::iter::repeat;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::Widget;

pub fn draw_card(val: char, count: usize) -> String {
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

fn char_of_digit(c: u8) -> char {
    match c {
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        _ => panic!("shouldn't need a char for this digit: {}", c)
    }
}

pub struct CardsWidget<'a>(pub &'a crate::mechanics::Cards, pub &'a crate::mechanics::GameState, pub bool);

impl<'a> CardsWidget<'a> {
    pub fn bounds(&self) -> (u16, u16) {
        (3*(self.0.len() as u16), 3)
    }
}
impl<'a> Widget for CardsWidget<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        for (card_idx , card) in self.0.iter().enumerate() {
            let color = if self.2 { tui::style::Color::Reset } else { color_from_place(card.0) };
            let card_text = draw_card(if self.2 { ' ' } else { char_of_digit(self.1.places[card.0 as usize].value) }, 1);
            for (count, line) in card_text
                .lines()
                .enumerate()
            {
                buf.set_string(area.left() + 3 * (card_idx as u16), area.top() + (count as u16), line, Style::default().fg(color));
            }
        }
    }
}
pub struct PlaceWidget<'a>(pub &'a crate::mechanics::Place, pub u8);

fn color_from_place(pl: u8) -> tui::style::Color {
    use tui::style::Color::*;
    match pl {
        0 => Red,
        1 => Green,
        2 => Yellow,
        3 => Blue,
        4 => Magenta,
        5 => Cyan,
        6 => Reset,
        _ => panic!("Too many places to color :(")
    }
}

impl<'a> Widget for PlaceWidget<'a> {
    // A place is built around 7x7 squares on top of each other, forming a 7x21 area.
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        // TODO: consider being more efficient.
        let top_square = draw_card(char_of_digit(self.0.value), self.0.scores[0] as usize);
        let main_square = format!(
            "\
             ╒═════╕\n\
             │{}    │\n\
             │     │\n\
             │    {}│\n\
             ╘═════╛",
            self.0.value, self.0.value
        );
        let bottom_square = draw_card(char_of_digit(self.0.value), self.0.scores[1] as usize);
        for (count, line) in top_square
            .lines()
            .chain(main_square.lines())
            .chain(bottom_square.lines())
            .enumerate()
        {
            let count = count as u16;
            buf.set_string(area.left(), area.top() + count, line, Style::default().fg(color_from_place(self.1)));
        }
    }
}
