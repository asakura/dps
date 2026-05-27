//! Component trait and per-screen implementations.

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use ratatui::{buffer::Buffer, widgets::TableState};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, config::Config, tui::Event};

use crate::theme::Theme;

pub mod fps;
pub mod hint_bar;
pub mod home;
pub mod mod_tab;
pub mod ppo2_tab;
pub mod which_key;

pub use fps::FpsCounter;
pub use home::Home;

/// Error produced by [`ComponentNew`] implementors.
///
/// # Examples
///
/// ```
/// use dps::components::ComponentError;
///
/// let e = ComponentError::InvalidState("no row selected");
/// assert_eq!(e.to_string(), "invalid component state: no row selected");
/// ```
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum ComponentError {
    /// The component received an action while in an invalid state (e.g. no row selected).
    #[error("invalid component state: {0}")]
    InvalidState(&'static str),
}

/// Convenience alias so trait method signatures stay concise.
pub type Result<T> = std::result::Result<T, ComponentError>;

/// A single key→action entry for the which-key popup and help line.
#[derive(Debug, Clone, Copy)]
pub struct KeyBinding {
    /// Key label shown in the popup (e.g. `"j/k"`, `"Enter"`).
    pub key: &'static str,
    /// Short description of the action.
    pub desc: &'static str,
}

/// Row delta applied by scroll-up/scroll-down navigation.
pub const SCROLL_DELTA: isize = 10;
/// Row delta applied by page-up/page-down navigation.
pub const PAGE_DELTA: isize = 20;

/// Moves the selected row by `delta`, clamping the result to `[0, max]`.
pub fn move_row(state: &mut TableState, delta: isize, max: usize) {
    let next = state
        .selected()
        .map_or(0, |i| i.saturating_add_signed(delta).min(max));

    state.select(Some(next));
}

/// Interface that every screen must implement to participate in the event loop
/// and render pipeline.
pub trait Component {
    /// Short display name shown in the tab bar.
    fn title(&self) -> &'static str;
    /// Draw the component's content into `area`.
    fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme);
    /// Render a one-line status bar below the main content area.
    fn render_status(&self, area: Rect, buf: &mut Buffer, theme: &Theme);
    /// Respond to a semantic action produced by the keybinding layer.
    ///
    /// Called when a configured key sequence resolves to an [`Action`] before
    /// the raw-key fallback path is reached.  The default implementation is a
    /// no-op; components override it for the actions they support.
    fn handle_action(&mut self, _action: Action) {}

    /// Structured key bindings for the which-key popup and hint line.
    fn key_bindings(&self) -> &'static [KeyBinding] {
        [].as_slice()
    }
}

/// Interface for visual and interactive UI elements in the [`AppNew`] event loop.
///
/// Implementors receive events, maintain state, and render themselves into the
/// terminal each frame.  Only [`draw`] is required; all other methods have
/// no-op defaults that can be overridden selectively.
///
/// # Lifecycle
///
/// [`AppNew`] calls trait methods in a fixed sequence:
///
/// **Startup (once per run):**
///
/// 1. [`register_action_handler`] — hands the component a
///    [`mpsc::UnboundedSender<Action>`] it can use to push actions at any
///    time, including from async tasks spawned later.
/// 2. [`register_config_handler`] — hands the component a clone of the
///    loaded [`Config`] so it can cache the active theme, key overrides, or
///    any other config fields it needs.
/// 3. [`init`] — called with the current terminal [`Size`] so the component
///    can pre-compute layout-dependent state before the first render.
///
/// **Per-iteration (repeated until quit):**
///
/// 4. [`handle_events`] — receives the raw [`Event`] from the TUI stream.
///    The default routes [`Key`] → [`handle_key_event`] and
///    [`Mouse`] → [`handle_mouse_event`].  Any returned [`Action`] is
///    forwarded to [`AppNew`]'s action channel.
/// 5. [`update`] — called once per queued [`Action`], for every action
///    dequeued by [`AppNew`] regardless of which component or system
///    produced it.  May return an [`Action`] to chain effects.
/// 6. [`draw`] — renders the component's current state into `area`.
///    Called only when a [`Render`] action fires; errors are caught by
///    [`AppNew`] and converted to [`Action::Error`] rather than propagating.
///
/// # Action channel
///
/// The sender received via [`register_action_handler`] can be stored and used
/// anywhere — from within [`update`], or from a background task spawned
/// during [`init`].  Every action pushed on it is processed by [`AppNew`] on
/// the next drain pass, and every component's [`update`] will see it.
///
/// # Event dispatch
///
/// [`handle_events`] is the single entry point for raw [`Event`]s.  Override
/// it only when you need to react to event variants beyond [`Key`] and
/// [`Mouse`] (e.g. [`FocusGained`]).  For ordinary key or mouse handling,
/// override [`handle_key_event`] or [`handle_mouse_event`] instead — the
/// default routing calls them automatically.
///
/// # Errors
///
/// All fallible methods return [`Result<T>`], an alias for
/// <code>std::result::Result<T, [ComponentError]></code>.  The only variant is
/// [`ComponentError::InvalidState`]; return it when an action or event arrives
/// in a state where it cannot be meaningfully handled (e.g. a scroll event
/// while no row is selected).
///
/// # Examples
///
/// Minimal component — only `draw` is required:
///
/// ```no_run
/// use dps::components::{ComponentNew, Result};
/// use ratatui::{Frame, layout::Rect, widgets::Paragraph};
///
/// struct Label(&'static str);
///
/// impl ComponentNew for Label {
///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
///         frame.render_widget(Paragraph::new(self.0), area);
///         Ok(())
///     }
/// }
/// ```
///
/// Component that stores the action sender, responds to keys, reacts to
/// actions, and renders stateful content:
///
/// ```no_run
/// use crossterm::event::{KeyCode, KeyEvent};
/// use tokio::sync::mpsc::UnboundedSender;
/// use dps::action::{Action, Movement};
/// use dps::components::{ComponentNew, Result};
/// use ratatui::{Frame, layout::Rect, widgets::Paragraph};
///
/// struct Counter {
///     count: u32,
///     tx: Option<UnboundedSender<Action>>,
/// }
///
/// impl ComponentNew for Counter {
///     fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
///         self.tx = Some(tx);
///         Ok(())
///     }
///
///     fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
///         Ok(match key.code {
///             KeyCode::Char('+') => Some(Action::Select),
///             _ => None,
///         })
///     }
///
///     fn update(&mut self, action: Action) -> Result<Option<Action>> {
///         if matches!(action, Action::Select) {
///             self.count += 1;
///         }
///         Ok(None)
///     }
///
///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
///         frame.render_widget(Paragraph::new(self.count.to_string()), area);
///         Ok(())
///     }
/// }
/// ```
///
/// [`AppNew`]: crate::app::AppNew
/// [`draw`]: ComponentNew::draw
/// [`register_action_handler`]: ComponentNew::register_action_handler
/// [`register_config_handler`]: ComponentNew::register_config_handler
/// [`init`]: ComponentNew::init
/// [`handle_events`]: ComponentNew::handle_events
/// [`handle_key_event`]: ComponentNew::handle_key_event
/// [`handle_mouse_event`]: ComponentNew::handle_mouse_event
/// [`update`]: ComponentNew::update
/// [`Key`]: crate::tui::Event::Key
/// [`Mouse`]: crate::tui::Event::Mouse
/// [`FocusGained`]: crate::tui::Event::FocusGained
/// [`Render`]: crate::action::Action::Render
/// [`mpsc::UnboundedSender<Action>`]: tokio::sync::mpsc::UnboundedSender
/// [`Size`]: ratatui::layout::Size
pub trait ComponentNew {
    /// Provides the component with a sender for dispatching [`Action`]s.
    ///
    /// The default is a no-op; override to store `tx` so the component can push
    /// actions from its own event handlers.
    ///
    /// # Errors
    ///
    /// The default returns `Ok(())` unconditionally; overrides may return `Err`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio::sync::mpsc::UnboundedSender;
    /// use dps::action::Action;
    /// use dps::components::{ComponentNew, Result};
    /// use ratatui::{Frame, layout::Rect};
    ///
    /// struct MyComponent {
    ///     tx: Option<UnboundedSender<Action>>,
    /// }
    ///
    /// impl ComponentNew for MyComponent {
    ///     fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    ///         self.tx = Some(tx);
    ///         Ok(())
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx; // to appease clippy
        Ok(())
    }

    /// Provides the component with the application [`Config`].
    ///
    /// The default is a no-op; override to cache theme colours, keybinding
    /// overrides, or any other config fields the component needs.
    ///
    /// # Errors
    ///
    /// The default returns `Ok(())` unconditionally; overrides may return `Err`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dps::config::Config;
    /// use dps::components::{ComponentNew, Result};
    /// use dps::theme::Theme;
    /// use ratatui::{Frame, layout::Rect};
    ///
    /// struct Styled {
    ///     theme: Option<Theme>,
    /// }
    ///
    /// impl ComponentNew for Styled {
    ///     fn register_config_handler(&mut self, config: Config) -> Result<()> {
    ///         self.theme = Some(*config.active_theme());
    ///         Ok(())
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        let _ = config; // to appease clippy

        Ok(())
    }

    /// Called once before the first [`draw`] with the terminal area size.
    ///
    /// Override to pre-allocate layout buffers or clamp scroll state to the
    /// visible height.
    ///
    /// [`draw`]: ComponentNew::draw
    ///
    /// # Errors
    ///
    /// The default returns `Ok(())` unconditionally; overrides may return `Err`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dps::components::{ComponentNew, Result};
    /// use ratatui::{Frame, layout::{Rect, Size}};
    ///
    /// struct Scrollable {
    ///     visible_rows: usize,
    /// }
    ///
    /// impl ComponentNew for Scrollable {
    ///     fn init(&mut self, area: Size) -> Result<()> {
    ///         self.visible_rows = area.height as usize;
    ///         Ok(())
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn init(&mut self, area: Size) -> Result<()> {
        let _ = area; // to appease clippy

        Ok(())
    }

    /// Dispatches an incoming TUI event to the appropriate handler.
    ///
    /// The default routes [`Event::Key`] → [`handle_key_event`] and
    /// [`Event::Mouse`] → [`handle_mouse_event`]; all other variants produce
    /// `None`.  Override only when the dispatch logic itself must change —
    /// for example, to react to [`Event::FocusGained`] or [`Event::FocusLost`].
    /// For ordinary key or mouse handling, override [`handle_key_event`] or
    /// [`handle_mouse_event`] instead.
    ///
    /// [`handle_key_event`]: ComponentNew::handle_key_event
    /// [`handle_mouse_event`]: ComponentNew::handle_mouse_event
    ///
    /// # Errors
    ///
    /// Propagates any `Err` returned by [`handle_key_event`] or [`handle_mouse_event`].
    ///
    /// # Examples
    ///
    /// Override to handle focus events in addition to the default key/mouse routing:
    ///
    /// ```no_run
    /// use dps::components::{ComponentNew, Result};
    /// use dps::tui::Event;
    /// use dps::action::Action;
    /// use ratatui::{Frame, layout::Rect};
    ///
    /// struct FocusAware { focused: bool }
    ///
    /// impl ComponentNew for FocusAware {
    ///     fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
    ///         match &event {
    ///             Some(Event::FocusGained) => { self.focused = true; return Ok(None); }
    ///             Some(Event::FocusLost)   => { self.focused = false; return Ok(None); }
    ///             _ => {}
    ///         }
    ///         // fall through to the standard key/mouse routing
    ///         Ok(match event {
    ///             Some(Event::Key(key))     => self.handle_key_event(key)?,
    ///             Some(Event::Mouse(mouse)) => self.handle_mouse_event(mouse)?,
    ///             _ => None,
    ///         })
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        let action = match event {
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };

        Ok(action)
    }

    /// Maps a raw key press to an [`Action`].
    ///
    /// Returns `None` by default. Override to intercept keys before they reach
    /// the global keybinding layer.
    ///
    /// # Errors
    ///
    /// The default returns `Ok(None)` unconditionally; overrides may return
    /// [`ComponentError::InvalidState`] if the key is received in an unexpected state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crossterm::event::{KeyCode, KeyEvent};
    /// use dps::action::{Action, Movement};
    /// use dps::components::{ComponentNew, Result};
    /// use ratatui::{Frame, layout::Rect};
    ///
    /// struct Nav;
    ///
    /// impl ComponentNew for Nav {
    ///     fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    ///         let action = match key.code {
    ///             KeyCode::Char('j') => Some(Action::Move(Movement::Down)),
    ///             KeyCode::Char('k') => Some(Action::Move(Movement::Up)),
    ///             _ => None,
    ///         };
    ///         Ok(action)
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        let _ = key; // to appease clippy
        Ok(None)
    }

    /// Maps a raw mouse event to an [`Action`].
    ///
    /// Returns `None` by default. Override to respond to scroll or click events.
    ///
    /// # Errors
    ///
    /// The default returns `Ok(None)` unconditionally; overrides may return
    /// [`ComponentError::InvalidState`] if the event is received in an unexpected state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crossterm::event::{MouseEvent, MouseEventKind};
    /// use dps::action::{Action, Movement};
    /// use dps::components::{ComponentNew, Result};
    /// use ratatui::{Frame, layout::Rect};
    ///
    /// struct Scrollable;
    ///
    /// impl ComponentNew for Scrollable {
    ///     fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
    ///         let action = match mouse.kind {
    ///             MouseEventKind::ScrollDown => Some(Action::Move(Movement::ScrollDown)),
    ///             MouseEventKind::ScrollUp => Some(Action::Move(Movement::ScrollUp)),
    ///             _ => None,
    ///         };
    ///         Ok(action)
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        let _ = mouse; // to appease clippy
        Ok(None)
    }

    /// Applies a semantic [`Action`] to the component's state.
    ///
    /// Called by [`AppNew`] for every action dequeued from the channel,
    /// regardless of which component or system produced it — infrastructure
    /// actions (`Tick`, `Render`, `Resize`, …) pass through here too.
    ///
    /// Returning `Some(action)` re-enqueues that action on the global channel,
    /// so effects can be chained: the returned action is processed by [`AppNew`]
    /// and fanned back out to every component's `update`.  Avoid returning an
    /// action unconditionally — it will cycle forever.
    ///
    /// Returns `None` by default.
    ///
    /// [`AppNew`]: crate::app::AppNew
    ///
    /// # Errors
    ///
    /// The default returns `Ok(None)` unconditionally; overrides may return
    /// [`ComponentError::InvalidState`] if the action is received in an unexpected state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dps::action::Action;
    /// use dps::components::{ComponentNew, Result};
    /// use ratatui::{Frame, layout::Rect};
    ///
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// impl ComponentNew for Counter {
    ///     fn update(&mut self, action: Action) -> Result<Option<Action>> {
    ///         if matches!(action, Action::Select) {
    ///             self.count += 1;
    ///         }
    ///         Ok(None)
    ///     }
    ///
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> { Ok(()) }
    /// }
    /// ```
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let _ = action; // to appease clippy
        Ok(None)
    }

    /// Renders the component into `area` on the current `frame`.
    ///
    /// This is the only required method.  Called each time [`AppNew`] processes
    /// an [`Action::Render`] (driven by the configured frame rate);
    /// implementations must complete quickly to keep the TUI responsive.
    ///
    /// Errors returned here are **not** propagated to the caller.  [`AppNew`]
    /// catches them and converts each one to an [`Action::Error`], keeping a
    /// single misbehaving component from aborting the whole render pass.
    ///
    /// [`AppNew`]: crate::app::AppNew
    /// [`Action::Render`]: crate::action::Action::Render
    /// [`Action::Error`]: crate::action::Action::Error
    ///
    /// # Errors
    ///
    /// Returns [`ComponentError`] if rendering cannot complete; the concrete
    /// condition is determined by the implementation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dps::components::{ComponentNew, Result};
    /// use ratatui::{Frame, layout::Rect, widgets::Paragraph};
    ///
    /// struct StatusLine(String);
    ///
    /// impl ComponentNew for StatusLine {
    ///     fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    ///         frame.render_widget(Paragraph::new(self.0.as_str()), area);
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()>;
}

#[cfg(test)]
/// Utilities for testing components.
pub mod test_utils {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    /// Renders a widget into a single-row buffer and returns its text content.
    pub fn widget_text(widget: impl Widget, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf);

        buf.content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }
}
