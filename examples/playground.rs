use turmoil::{components::Frame, prelude::*};

struct App {
    uppercase: bool,
    frame: Frame,
    offset: (i32, i32),
}

impl App {
    pub fn new() -> Self {
        Self {
            uppercase: true,
            frame: Frame::default(),
            offset: (0, 0),
        }
    }
}

impl Component for App {
    fn key_event(&mut self, key: KeyCode, _mods: KeyModifiers) -> bool {
        match key {
            KeyCode::Char('i') => self.uppercase = !self.uppercase,
            KeyCode::Left => self.offset.0 -= 1,
            KeyCode::Right => self.offset.0 += 1,
            KeyCode::Up => self.offset.1 -= 1,
            KeyCode::Down => self.offset.1 += 1,
            _ => return false,
        }

        true
    }

    fn draw(&self, area: Rect, buf: &mut BoundedBuffer<'_>) {
        self.frame.draw(area, buf);

        for (i, c) in "hello world".chars().enumerate() {
            buf.get_mut((i as i32 + self.offset.0) as u16, self.offset.1 as u16)
                .set_char(if self.uppercase {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                });
        }
    }
}

fn main() {
    turmoil::run(App::new()).unwrap();
}
