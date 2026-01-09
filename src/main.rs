use std::io;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};

fn main() -> anyhow::Result<()> {
    let mut gs = multiway_tug::mechanics::GameState::new(rand::SeedableRng::from_entropy());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // TODO:
    // 1. Make the place cards indicate control (shift them towards the controlling player?)
    // 2. Draw our player always on the bottom
    // 3. Draw remaining_actions
    // 4. User input for action selection
    loop {
        terminal.draw(|f| {
            let other_player_cards =
                multiway_tug::CardsWidget(&gs.players[0].hand, &gs, true);
            let our_cards = multiway_tug::CardsWidget(&gs.players[1].hand, &gs, false);
            let (ox, oy) = multiway_tug::CardsWidget(&gs.players[0].hand, &gs, true).bounds();
            f.render_widget(other_player_cards, Rect::new(24 - (ox / 2), 0, ox, oy));
            for (place_idx, place) in gs.places.iter().enumerate() {
                let place_rect = Rect::new(7 * (place_idx as u16), 3, 7, 21);
                f.render_widget(
                    multiway_tug::PlaceWidget(place, place_idx as u8),
                    place_rect,
                );
            }
            let (ox, oy) = multiway_tug::CardsWidget(&gs.players[1].hand, &gs, false).bounds();
            f.render_widget(our_cards, Rect::new(24 - (ox / 2), 14, ox, oy));
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if let Some(v) = c.to_digit(10) {
                            if (v as usize) < gs.places.len() {
                                use rand::Rng;
                                let idx: usize = gs.rng.gen_range(0..2);
                                gs.places[v as usize].scores[idx] += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
