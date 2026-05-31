//! Tab pane: manages tab switching between [`ModTab`] and [`PpO2Tab`].
//!
//! # Examples
//!
//! ```
//! use dps::components::TabPane;
//!
//! let _pane = TabPane::new();
//! ```

use super::{ComponentNew, Result, mod_tab::ModTab, ppo2_tab::PpO2Tab};

use crate::{
    action::{Action, TabMotion},
    config::Config,
    registers::RegisterStore,
    theme::Theme,
};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect, Size},
    widgets::Tabs,
};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
enum ActiveTab {
    Mod,
    PpO2,
}

/// Tab pane: owns both application tabs and routes [`Action::Tab`] actions.
#[derive(Debug)]
pub struct TabPane {
    mod_tab: ModTab,
    ppo2_tab: PpO2Tab,
    active: ActiveTab,
    theme: Theme,
}

impl TabPane {
    /// Creates a `TabPane` with the MOD tab active.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::components::TabPane;
    ///
    /// let pane = TabPane::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            mod_tab: ModTab::new(),
            ppo2_tab: PpO2Tab::new(),
            active: ActiveTab::Mod,
            theme: Theme::default(),
        }
    }
}

impl Default for TabPane {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentNew for TabPane {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.mod_tab.register_action_handler(tx.clone())?;
        self.ppo2_tab.register_action_handler(tx)?;

        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.theme = *config.active_theme();
        self.mod_tab.register_config_handler(config.clone())?;
        self.ppo2_tab.register_config_handler(config)?;

        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        self.mod_tab.init(area)?;
        self.ppo2_tab.init(area)?;

        Ok(())
    }

    fn update(&mut self, action: Action, registers: &mut RegisterStore) -> Result<Option<Action>> {
        match action {
            Action::Tab(TabMotion::Next) => {
                self.active = match self.active {
                    ActiveTab::Mod => ActiveTab::PpO2,
                    ActiveTab::PpO2 => ActiveTab::Mod,
                };
            }
            Action::Tab(TabMotion::Prev) => {
                self.active = match self.active {
                    ActiveTab::PpO2 => ActiveTab::Mod,
                    ActiveTab::Mod => ActiveTab::PpO2,
                };
            }
            Action::Tab(TabMotion::GoTo(1)) => self.active = ActiveTab::Mod,
            Action::Tab(TabMotion::GoTo(2)) => self.active = ActiveTab::PpO2,
            Action::Tab(TabMotion::GoTo(_)) => {}
            _ => {
                return match self.active {
                    ActiveTab::Mod => self.mod_tab.update(action, registers),
                    ActiveTab::PpO2 => self.ppo2_tab.update(action, registers),
                };
            }
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
        let [tab_bar, content] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
        let active_idx = match self.active {
            ActiveTab::Mod => 0,
            ActiveTab::PpO2 => 1,
        };

        frame.render_widget(
            Tabs::new([ModTab::TITLE, PpO2Tab::TITLE]).select(active_idx),
            tab_bar,
        );

        match self.active {
            ActiveTab::Mod => self.mod_tab.draw(frame, content),
            ActiveTab::PpO2 => self.ppo2_tab.draw(frame, content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::action::{Action, TabMotion};
    use crate::components::ComponentError;
    use crate::registers::RegisterStore;

    use rstest::rstest;

    mod new {
        use super::*;

        #[rstest]
        fn active_is_mod_tab_on_construction() {
            let pane = TabPane::new();
            assert!(matches!(pane.active, ActiveTab::Mod));
        }
    }

    mod tab_switching {
        use super::*;

        #[rstest]
        fn tab_next_toggles_from_mod_to_ppo2() -> std::result::Result<(), ComponentError> {
            let mut pane = TabPane::new();

            pane.update(Action::Tab(TabMotion::Next), &mut RegisterStore::default())?;

            assert!(matches!(pane.active, ActiveTab::PpO2));

            Ok(())
        }

        #[rstest]
        fn tab_prev_toggles_from_ppo2_to_mod() -> std::result::Result<(), ComponentError> {
            let mut pane = TabPane::new();

            pane.active = ActiveTab::PpO2;
            pane.update(Action::Tab(TabMotion::Prev), &mut RegisterStore::default())?;

            assert!(matches!(pane.active, ActiveTab::Mod));

            Ok(())
        }

        #[rstest]
        fn tab_goto_1_selects_mod() -> std::result::Result<(), ComponentError> {
            let mut pane = TabPane::new();

            pane.active = ActiveTab::PpO2;
            pane.update(
                Action::Tab(TabMotion::GoTo(1)),
                &mut RegisterStore::default(),
            )?;

            assert!(matches!(pane.active, ActiveTab::Mod));

            Ok(())
        }

        #[rstest]
        fn tab_goto_2_selects_ppo2() -> std::result::Result<(), ComponentError> {
            let mut pane = TabPane::new();

            pane.update(
                Action::Tab(TabMotion::GoTo(2)),
                &mut RegisterStore::default(),
            )?;

            assert!(matches!(pane.active, ActiveTab::PpO2));

            Ok(())
        }

        #[rstest]
        fn tab_goto_out_of_range_is_noop() -> std::result::Result<(), ComponentError> {
            let mut pane = TabPane::new();

            pane.update(
                Action::Tab(TabMotion::GoTo(99)),
                &mut RegisterStore::default(),
            )?;

            assert!(matches!(pane.active, ActiveTab::Mod));

            Ok(())
        }
    }
}
