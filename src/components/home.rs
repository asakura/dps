//! Home screen component — placeholder for the main application view.

use super::{ComponentNew, Result};

use crate::{action::Action, config::Config, registers::RegisterStore};

use ratatui::{prelude::*, widgets::Paragraph};
use tokio::sync::mpsc::UnboundedSender;

/// Main home screen.
#[derive(Debug, Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
}

impl Home {
    /// Creates a new `Home` with no action sender or config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ComponentNew for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);

        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;

        Ok(())
    }

    fn update(&mut self, action: Action, _registers: &mut RegisterStore) -> Result<Option<Action>> {
        let _ = action;

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
        frame.render_widget(Paragraph::new("hello world"), area);

        Ok(())
    }
}
