use std::{
    io::{Stdout, stdout},
    ops::{Deref, DerefMut},
    time::Duration,
};

use crossterm::{
    cursor,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind, MouseEvent,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        is_raw_mode_enabled,
    },
};
use futures::{FutureExt, StreamExt};
use ratatui::{Terminal, backend::CrosstermBackend as Backend};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::interval,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

/// Events produced by the terminal input stream and the tick/render timers.
///
/// [`App::run`] receives these through an unbounded channel and dispatches
/// them to the appropriate handler. All variants are serialisable so they can
/// be injected via [`Tui::event_tx`] in tests or from external signal handlers.
///
/// [`App::run`]: crate::app::App::run
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// Fired once immediately after the event loop starts.
    Init,
    /// Signals the application to exit cleanly.
    Quit,
    /// A non-fatal I/O error occurred reading the event stream.
    Error,
    /// The event stream has closed and will produce no further events.
    Closed,
    /// Periodic timer tick at the configured `tick_rate`.
    Tick,
    /// Periodic render request at the configured `frame_rate`.
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

/// RAII guard that owns the terminal and drives the async event loop.
///
/// Use the builder methods to configure rates and optional features, then call
/// [`enter`] to start. The terminal is restored automatically on [`drop`].
///
/// # Examples
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> color_eyre::Result<()> {
/// use dps::tui::Tui;
///
/// let mut tui = Tui::new()?
///     .tick_rate(4.0)
///     .frame_rate(60.0);
/// tui.enter()?;
/// while let Some(event) = tui.next_event().await {
///     // handle event
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`enter`]: Tui::enter
pub struct Tui {
    terminal: Terminal<Backend<Stdout>>,
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
    event_rx: UnboundedReceiver<Event>,
    /// Inject a synthetic event into the loop (e.g. from tests or signal handlers).
    pub event_tx: UnboundedSender<Event>,
    frame_rate: f64,
    tick_rate: f64,
    mouse: bool,
    paste: bool,
}

impl Tui {
    /// Creates a `Tui` with default rates (4 Hz tick, 60 Hz render).
    ///
    /// Does **not** enter raw mode — call [`enter`] when ready to start the
    /// event loop. The terminal backend is initialised immediately so that
    /// [`draw`] can be called once raw mode is active.
    ///
    /// [`enter`]: Tui::enter
    /// [`draw`]: ratatui::Terminal::draw
    pub fn new() -> color_eyre::Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            terminal: ratatui::Terminal::new(Backend::new(stdout()))?,
            task: tokio::spawn(async {}),
            cancellation_token: CancellationToken::new(),
            event_rx,
            event_tx,
            frame_rate: 60.0,
            tick_rate: 4.0,
            mouse: false,
            paste: false,
        })
    }

    /// Sets the application-logic timer rate in Hz (default 4.0).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dps::tui::Tui;
    /// let tui = Tui::new()?.tick_rate(10.0);
    /// # Ok::<_, color_eyre::Report>(())
    /// ```
    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        assert!(tick_rate > 0.0 && tick_rate.is_finite(), "tick_rate must be a positive finite number");
        self.tick_rate = tick_rate;
        self
    }

    /// Sets the render timer rate in Hz (default 60.0).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dps::tui::Tui;
    /// let tui = Tui::new()?.frame_rate(30.0);
    /// # Ok::<_, color_eyre::Report>(())
    /// ```
    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        assert!(frame_rate > 0.0 && frame_rate.is_finite(), "frame_rate must be a positive finite number");
        self.frame_rate = frame_rate;
        self
    }

    /// Enables or disables mouse-event capture (default `false`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dps::tui::Tui;
    /// let tui = Tui::new()?.mouse(true);
    /// # Ok::<_, color_eyre::Report>(())
    /// ```
    pub fn mouse(mut self, mouse: bool) -> Self {
        self.mouse = mouse;
        self
    }

    /// Enables or disables bracketed-paste support (default `false`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dps::tui::Tui;
    /// let tui = Tui::new()?.paste(true);
    /// # Ok::<_, color_eyre::Report>(())
    /// ```
    pub fn paste(mut self, paste: bool) -> Self {
        self.paste = paste;
        self
    }

    /// Spawns the async event loop without entering raw mode.
    ///
    /// Prefer [`enter`] for normal use; call `start` directly only when you
    /// need to manage raw mode yourself.  Any previously running loop is
    /// cancelled first.
    ///
    /// [`enter`]: Tui::enter
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.start();
    /// tui.stop()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start(&mut self) {
        assert!(
            self.frame_rate >= self.tick_rate,
            "frame_rate ({}) should be >= tick_rate ({})",
            self.frame_rate,
            self.tick_rate
        );
        self.cancel();
        self.cancellation_token = CancellationToken::new();

        let event_loop = Self::event_loop(
            self.event_tx.clone(),
            self.cancellation_token.clone(),
            self.tick_rate,
            self.frame_rate,
        );

        self.task = tokio::spawn(async {
            event_loop.await;
        });
    }

    async fn event_loop(
        event_tx: UnboundedSender<Event>,
        cancellation_token: CancellationToken,
        tick_rate: f64,
        frame_rate: f64,
    ) {
        let mut event_stream = EventStream::new();
        let mut tick_interval = interval(Duration::from_secs_f64(1.0 / tick_rate));
        let mut render_interval = interval(Duration::from_secs_f64(1.0 / frame_rate));

        // if this fails, then it's likely a bug in the calling code
        event_tx
            .send(Event::Init)
            .expect("failed to send init event");

        loop {
            let event = tokio::select! {
                _ = cancellation_token.cancelled() => {
                    break;
                }
                _ = tick_interval.tick() => Event::Tick,
                _ = render_interval.tick() => Event::Render,

                crossterm_event = event_stream.next().fuse() => match crossterm_event {
                    Some(Ok(event)) => match event {
                        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => Event::Key(key),
                        CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                        CrosstermEvent::Resize(x, y) => Event::Resize(x, y),
                        CrosstermEvent::FocusLost => Event::FocusLost,
                        CrosstermEvent::FocusGained => Event::FocusGained,
                        CrosstermEvent::Paste(s) => Event::Paste(s),
                        _ => continue,
                    }
                    Some(Err(e)) => {
                        warn!("crossterm event error: {e}");
                        Event::Error
                    }
                    None => break,
                },
            };

            if event_tx.send(event).is_err() {
                break;
            }
        }

        cancellation_token.cancel();
    }

    /// Cancels the event loop and waits for its task to finish.
    ///
    /// Blocks the calling thread via [`tokio::task::block_in_place`] for up to
    /// ~100 ms; aborts the task if it hasn't exited by then.  Requires the
    /// multi-threaded Tokio runtime.
    pub fn stop(&self) -> color_eyre::Result<()> {
        self.cancel();

        tokio::task::block_in_place(|| {
            let mut counter = 0;
            while !self.task.is_finished() {
                std::thread::sleep(Duration::from_millis(1));
                counter += 1;

                if counter == 51 {
                    self.task.abort();
                }

                if counter > 100 {
                    error!("Failed to abort task in 100 milliseconds for unknown reason");
                    break;
                }
            }
        });

        Ok(())
    }

    /// Enters the alternate screen, enables raw mode, and starts the event loop.
    ///
    /// Call [`exit`] (or let [`Drop`] handle it) to restore the terminal.
    ///
    /// [`exit`]: Tui::exit
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.enter()?;
    /// tui.exit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn enter(&mut self) -> color_eyre::Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;

        if self.mouse {
            execute!(stdout(), EnableMouseCapture)?;
        }

        if self.paste {
            execute!(stdout(), EnableBracketedPaste)?;
        }

        self.start();

        Ok(())
    }

    /// Stops the event loop and restores the terminal to its original state.
    ///
    /// Safe to call even if [`enter`] was never called; the raw-mode guard
    /// checks the terminal state before attempting to restore it.
    ///
    /// [`enter`]: Tui::enter
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.enter()?;
    /// tui.exit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn exit(&mut self) -> color_eyre::Result<()> {
        self.stop()?;

        if is_raw_mode_enabled()? {
            self.flush()?;

            if self.paste {
                execute!(stdout(), DisableBracketedPaste)?;
            }

            if self.mouse {
                execute!(stdout(), DisableMouseCapture)?;
            }

            execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;

            disable_raw_mode()?;
        }

        Ok(())
    }

    /// Signals the event loop to stop; returns immediately without waiting.
    ///
    /// Use [`stop`] if you need to wait for the task to finish.
    ///
    /// [`stop`]: Tui::stop
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.start();
    /// tui.cancel();
    /// # Ok(())
    /// # }
    /// ```
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Exits the TUI and suspends the process with `SIGTSTP` (Unix only).
    ///
    /// Call [`resume`] to re-enter the TUI after the process is foregrounded.
    ///
    /// [`resume`]: Tui::resume
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.enter()?;
    /// tui.suspend()?; // process receives SIGTSTP here
    /// # Ok(())
    /// # }
    /// ```
    pub fn suspend(&mut self) -> color_eyre::Result<()> {
        self.exit()?;

        #[cfg(not(windows))]
        signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP)?;

        Ok(())
    }

    /// Re-enters the TUI after a [`suspend`].
    ///
    /// [`suspend`]: Tui::suspend
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.enter()?;
    /// tui.suspend()?;
    /// tui.resume()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn resume(&mut self) -> color_eyre::Result<()> {
        self.enter()?;
        Ok(())
    }

    /// Receives the next event from the event loop, or `None` if the channel is closed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main(flavor = "multi_thread")]
    /// # async fn main() -> color_eyre::Result<()> {
    /// # use dps::tui::Tui;
    /// let mut tui = Tui::new()?;
    /// tui.start();
    /// if let Some(event) = tui.next_event().await {
    ///     // handle event
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_event(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<Backend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if let Err(e) = self.exit() {
            error!("failed to exit terminal: {e:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn succeeds_without_tty() {
            // Terminal::new does not require a real TTY; only enter() does.
            assert!(Tui::new().is_ok());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn default_rates() {
            let tui = Tui::new().unwrap();
            assert_eq!(tui.tick_rate, 4.0);
            assert_eq!(tui.frame_rate, 60.0);
            assert!(!tui.mouse);
            assert!(!tui.paste);
        }
    }

    mod builder {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn tick_rate_sets_value() {
            let tui = Tui::new().unwrap().tick_rate(10.0);
            assert_eq!(tui.tick_rate, 10.0);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn frame_rate_sets_value() {
            let tui = Tui::new().unwrap().frame_rate(30.0);
            assert_eq!(tui.frame_rate, 30.0);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn mouse_sets_value() {
            let tui = Tui::new().unwrap().mouse(true);
            assert!(tui.mouse);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn paste_sets_value() {
            let tui = Tui::new().unwrap().paste(true);
            assert!(tui.paste);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn builder_chain() {
            let tui = Tui::new()
                .unwrap()
                .tick_rate(10.0)
                .frame_rate(30.0)
                .mouse(true)
                .paste(true);
            assert_eq!(tui.tick_rate, 10.0);
            assert_eq!(tui.frame_rate, 30.0);
            assert!(tui.mouse);
            assert!(tui.paste);
        }
    }

    mod stop {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_ok_and_task_finishes() {
            let mut tui = Tui::new().unwrap();
            tui.start();
            assert!(tui.stop().is_ok());
            assert!(tui.task.is_finished());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn idempotent_on_already_stopped_tui() {
            let mut tui = Tui::new().unwrap();
            tui.start();
            assert!(tui.stop().is_ok());
            assert!(tui.stop().is_ok());
        }
    }

    mod event {
        use super::*;

        #[test]
        fn unit_variants_serialize_as_strings() {
            assert_eq!(serde_json::to_string(&Event::Tick).unwrap(), r#""Tick""#);
            assert_eq!(
                serde_json::to_string(&Event::Render).unwrap(),
                r#""Render""#
            );
            assert_eq!(serde_json::to_string(&Event::Init).unwrap(), r#""Init""#);
            assert_eq!(serde_json::to_string(&Event::Quit).unwrap(), r#""Quit""#);
        }

        #[test]
        fn tuple_variants_serialize_as_objects() {
            let json = serde_json::to_string(&Event::Paste("hello".into())).unwrap();
            assert_eq!(json, r#"{"Paste":"hello"}"#);

            let json = serde_json::to_string(&Event::Resize(80, 24)).unwrap();
            assert_eq!(json, r#"{"Resize":[80,24]}"#);
        }

        #[test]
        fn round_trips_via_json() {
            for event in [
                Event::Init,
                Event::Tick,
                Event::Render,
                Event::Quit,
                Event::Error,
                Event::Closed,
                Event::FocusGained,
                Event::FocusLost,
                Event::Paste("x".into()),
                Event::Resize(100, 50),
            ] {
                let json = serde_json::to_string(&event).unwrap();
                let _: Event = serde_json::from_str(&json).unwrap();
            }
        }
    }
}
