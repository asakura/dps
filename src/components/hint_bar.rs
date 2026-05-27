//! One-line hint bar showing component bindings followed by global bindings.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

use super::KeyBinding;

/// One-line bar that renders component-specific bindings followed by global bindings.
#[derive(Debug)]
pub struct HintBar<'a> {
    component: &'a [KeyBinding],
    global: &'a [KeyBinding],
    theme: Theme,
}

impl<'a> HintBar<'a> {
    /// Creates a new `HintBar`.
    #[must_use]
    pub const fn new(component: &'a [KeyBinding], global: &'a [KeyBinding], theme: Theme) -> Self {
        Self {
            component,
            global,
            theme,
        }
    }
}

impl Widget for HintBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hint = self
            .component
            .iter()
            .chain(self.global.iter())
            .map(|b| format!("{} {}", b.key, b.desc))
            .collect::<Vec<_>>()
            .join("   ");
        Paragraph::new(format!(" {hint}"))
            .style(self.theme.hint())
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_utils::widget_text;
    use color_eyre::Result;
    use color_eyre::eyre::eyre;

    static COMP: &[KeyBinding] = [KeyBinding {
        key: "j/k",
        desc: "move",
    }]
    .as_slice();
    static GLOB: &[KeyBinding] = [KeyBinding {
        key: "q",
        desc: "quit",
    }]
    .as_slice();

    #[test]
    fn renders_component_bindings_first() -> Result<()> {
        let text = widget_text(HintBar::new(COMP, GLOB, Theme::default()), 60);
        let j_pos = text
            .find("j/k")
            .ok_or_else(|| eyre!("'j/k' not found in hint bar text"))?;
        let q_pos = text
            .find("q quit")
            .ok_or_else(|| eyre!("'q quit' not found in hint bar text"))?;

        assert!(j_pos < q_pos);

        Ok(())
    }

    #[test]
    fn renders_all_bindings() {
        let text = widget_text(HintBar::new(COMP, GLOB, Theme::default()), 60);
        assert!(text.contains("j/k move"));
        assert!(text.contains("q quit"));
    }

    #[test]
    fn empty_bindings_renders_without_panic() {
        widget_text(
            HintBar::new([].as_slice(), [].as_slice(), Theme::default()),
            40,
        );
    }
}
