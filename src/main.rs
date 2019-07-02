extern crate multiway_tug;
extern crate termion;
extern crate tui;

use rand::Rng;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::Rect;
use tui::Terminal;

use std::io::Read;

struct TuiBehavior();

fn main() -> Result<(), failure::Error> {
    let mut gs = multiway_tug::mechanics::GameState::new(Box::new(rand::thread_rng()));

    let stdout = std::io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut read_buf = [0u8; 16];
    loop {
        terminal.draw(|mut f| {
            for (place_idx, place) in gs.places.iter().enumerate() {
                let place_rect = Rect::new(7 * (place_idx as u16), 0, 7, 21);
                f.render(&mut multiway_tug::PlaceWidget(place), place_rect);
            }
        })?;
        stdin.read(&mut read_buf)?;
        match std::str::from_utf8(&read_buf[0..1])?.parse::<usize>() {
            Ok(v) =>  {
                let idx: usize = gs.rng.gen_range(0, 2);
                gs.places[v].scores[idx] += 1;
            },
            e => { println!("Error: {:?}", e); },
        }
        if read_buf[0] == b'q' {
            break;
        }
    }

    Ok(())
}
