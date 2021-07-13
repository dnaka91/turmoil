#![deny(unsafe_code, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

use std::{
    io::{self, Stdout, Write},
    thread,
    time::Duration,
};

use crossbeam_channel::{select, Receiver};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::layout::Rect;
use tui::{backend::CrosstermBackend, widgets::Widget};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("")]
    Io(#[from] std::io::Error),
    #[error("")]
    Crossterm(#[from] crossterm::ErrorKind),
    #[error("")]
    Receive(#[from] crossbeam_channel::RecvError),
}

pub struct Terminal(tui::Terminal<CrosstermBackend<BufferWrapper<Stdout>>>);

struct BufferWrapper<W: Write>(W);

impl<W: Write> BufferWrapper<W> {
    fn new(mut output: W) -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        execute!(output, EnterAlternateScreen)?;

        Ok(Self(output))
    }
}

impl<W: Write> Drop for BufferWrapper<W> {
    fn drop(&mut self) {
        execute!(self.0, LeaveAlternateScreen).expect("switch to main screen");
        crossterm::terminal::disable_raw_mode().expect("disable raw mode");
    }
}

impl<W: Write> Write for BufferWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub fn terminal() -> Result<Terminal> {
    let stdout = BufferWrapper::new(io::stdout())?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = tui::Terminal::new(backend)?;

    Ok(Terminal(terminal))
}

#[must_use]
pub fn events() -> Receiver<Event> {
    let (tx, rx) = crossbeam_channel::bounded(0);

    thread::spawn(move || {
        while let Ok(event) = crossterm::event::read() {
            tx.send(event).ok();
        }
    });

    rx
}

pub fn run<T: Component>(mut main: T) -> Result<()> {
    let mut terminal = terminal()?;
    let events = events();

    let ticker = crossbeam_channel::tick(Duration::from_secs(1));

    'main: loop {
        terminal.0.draw(|f| {
            f.render_widget(ComponentGlue(&mut main), f.size());
        })?;

        select! {
            recv(ticker) -> _ => {},
            recv(events) -> event => {
                let event = match event {
                    Ok(e) => e,
                    Err(e) => break 'main Err(e.into()),
                };

                if !handle_component_event(&mut main, event) && handle_global_event(event) {
                    break 'main Ok(());
                }
            },
        }
    }
}

fn handle_component_event<T: Component>(main: &mut T, event: Event) -> bool {
    match event {
        Event::Key(KeyEvent { code, modifiers }) => main.key_event(code, modifiers),
        Event::Mouse(m) => main.mouse_event(m),
        Event::Resize(_, _) => false,
    }
}

fn handle_global_event(event: Event) -> bool {
    matches!(
        event,
        Event::Key(KeyEvent {
            code: KeyCode::Esc,
            ..
        })
    )
}

pub struct BoundedBuffer<'a>(&'a mut tui::buffer::Buffer);

impl<'a> BoundedBuffer<'a> {
    pub fn get_mut(&mut self, x: u16, y: u16) -> BoundedCell<'_> {
        BoundedCell(
            (x < self.0.area().right() && y < self.0.area.bottom())
                .then(move || self.0.get_mut(x, y)),
        )
    }
}

pub struct BoundedCell<'a>(Option<&'a mut tui::buffer::Cell>);

impl<'a> BoundedCell<'a> {
    pub fn set_char(&mut self, ch: char) {
        if let Some(cell) = self.0.as_mut() {
            cell.set_char(ch);
        }
    }
}

pub trait Component {
    fn key_event(&mut self, _key: KeyCode, _mods: KeyModifiers) -> bool {
        false
    }

    fn mouse_event(&mut self, _event: MouseEvent) -> bool {
        false
    }

    fn draw(&self, area: Rect, buf: &mut BoundedBuffer<'_>);
}

struct ComponentGlue<'a, T: Component>(&'a mut T);

impl<'a, T: Component> Widget for ComponentGlue<'a, T> {
    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer) {
        self.0.draw(area, &mut BoundedBuffer(buf));
    }
}

pub mod prelude {
    pub use crate::{BoundedBuffer, BoundedCell, Component};
    pub use crossterm::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
    };
    pub use tui::layout::Rect;
}

pub mod components {
    use tui::widgets::{Block, Borders, Widget};

    use crate::{BoundedBuffer, Component, Rect};

    pub struct Frame(Block<'static>);

    impl Default for Frame {
        fn default() -> Self {
            Self(Block::default().borders(Borders::ALL))
        }
    }

    impl Component for Frame {
        fn draw(&self, area: Rect, buf: &mut BoundedBuffer<'_>) {
            self.0.clone().render(area, buf.0);
        }
    }
}
