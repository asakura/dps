//! Which-key popup: Magit-style transient panel at the bottom of the screen.
//!
//! Bindings are shown in a column-major grid. The column count grows with the
//! terminal width; each column is at least 20 cells wide (1 lead + 7 key + 2 gap +
//! 10 description). The panel spans the full terminal width and is anchored to the
//! bottom edge.
//!
//! Each binding is rendered by a private `Entry` widget, which lays out a
//! single row as `LEAD | KEY_W | ENTRY_GAP | desc`.
//!
//! # Example
//!
//! ```no_run
//! use ratatui::{Terminal, backend::TestBackend};
//! use dps::components::{KeyBinding, which_key::WhichKey};
//!
//! static BINDINGS: [KeyBinding; 1] = [KeyBinding { key: "?", desc: "help" }];
//!
//! let backend = TestBackend::new(80, 24);
//! let mut terminal = Terminal::new(backend).unwrap();
//! terminal.draw(|f| {
//!     f.render_widget(WhichKey::new(&BINDINGS, &[]), f.area());
//! }).unwrap();
//! ```

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Clear, Paragraph, Widget},
};

use crate::theme::THEME;

use super::KeyBinding;

/// Key column width.
const KEY_W: u16 = 7;
/// Gap between key and description within one entry.
const ENTRY_GAP: u16 = 2;
/// Leading space before each entry.
const LEAD: u16 = 1;
/// Gap between columns.
const COL_GAP: u16 = 4;
/// Minimum description width used to calculate how many columns fit.
const MIN_DESC_W: u16 = 10;

/// Which-key popup widget.
///
/// Merges `global` and `component` bindings into a dynamic column grid anchored
/// to the bottom of the area it is rendered into. Pass the full terminal area so
/// the popup is positioned at the screen bottom.
pub struct WhichKey {
    global: &'static [KeyBinding],
    component: &'static [KeyBinding],
}

impl WhichKey {
    /// Creates a new `WhichKey` popup combining global and component bindings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dps::components::{KeyBinding, which_key::WhichKey};
    ///
    /// static GLOBAL: [KeyBinding; 1] = [KeyBinding { key: "?", desc: "help" }];
    /// static COMP: [KeyBinding; 1] = [KeyBinding { key: "q", desc: "quit" }];
    ///
    /// let _ = WhichKey::new(&GLOBAL, &COMP);
    /// ```
    #[must_use]
    pub fn new(global: &'static [KeyBinding], component: &'static [KeyBinding]) -> Self {
        Self { global, component }
    }
}

impl Widget for WhichKey {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let all: Vec<&KeyBinding> = self.global.iter().chain(self.component.iter()).collect();
        let n = all.len();

        if n == 0 {
            return;
        }

        // How many columns fit? Each column needs at least min_col_w cells; columns
        // after the first each cost an additional COL_GAP.
        let min_col_w = LEAD + KEY_W + ENTRY_GAP + MIN_DESC_W;
        let cols = ((area.width + COL_GAP) / (min_col_w + COL_GAP)).max(1) as usize;
        let rows = n.div_ceil(cols);
        let popup_h = u16::try_from(rows).unwrap_or(u16::MAX).min(area.height);
        let popup = bottom_rect(popup_h, area);

        Clear.render(popup, buf);
        buf.set_style(popup, THEME.popup_surface());

        let col_rects = Layout::horizontal(vec![Constraint::Fill(1); cols])
            .spacing(COL_GAP)
            .split(popup);

        for (col_idx, col_rect) in col_rects.iter().enumerate() {
            let row_rects = Layout::vertical(vec![Constraint::Length(1); rows]).split(*col_rect);

            for row in 0..rows {
                if let Some(b) = all.get(col_idx * rows + row) {
                    Entry(b).render(row_rects[row], buf);
                }
            }
        }
    }
}

/// [`Widget`] that renders one [`KeyBinding`] as a `LEAD | KEY_W | ENTRY_GAP | desc` row.
struct Entry<'a>(&'a KeyBinding);

impl Widget for Entry<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [_lead, key_area, _gap, desc_area] = Layout::horizontal([
            Constraint::Length(LEAD),
            Constraint::Length(KEY_W),
            Constraint::Length(ENTRY_GAP),
            Constraint::Fill(1),
        ])
        .areas(area);

        Paragraph::new(self.0.key)
            .style(THEME.key_label())
            .render(key_area, buf);

        Paragraph::new(self.0.desc)
            .style(THEME.body_text())
            .render(desc_area, buf);
    }
}

/// A `Rect` spanning the full width of `area`, `height` rows tall, anchored to the bottom edge.
fn bottom_rect(height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(height),
        width: area.width,
        height: height.min(area.height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, layout::Position};

    mod bottom_rect {
        use super::*;

        #[test]
        fn anchors_to_bottom() {
            let area = Rect::new(0, 0, 40, 10);
            assert_eq!(bottom_rect(3, area), Rect::new(0, 7, 40, 3));
        }

        #[test]
        fn full_height_fills_area() {
            let area = Rect::new(0, 0, 40, 10);
            assert_eq!(bottom_rect(10, area), Rect::new(0, 0, 40, 10));
        }

        #[test]
        fn clamps_to_area_height() {
            let area = Rect::new(0, 0, 40, 5);
            assert_eq!(bottom_rect(10, area), Rect::new(0, 0, 40, 5));
        }

        #[test]
        fn preserves_area_offset() {
            // Non-zero origin: popup must be at the bottom of that sub-area, not the screen.
            let area = Rect::new(5, 2, 30, 8);
            assert_eq!(bottom_rect(2, area), Rect::new(5, 8, 30, 2));
        }
    }

    mod entry {
        use super::*;

        static BINDING: KeyBinding = KeyBinding {
            key: "?",
            desc: "zig",
        };
        static LONG_KEY: KeyBinding = KeyBinding {
            key: "toolongkey",
            desc: "d",
        };

        fn render_entry(b: &'static KeyBinding, width: u16) -> ratatui::buffer::Buffer {
            let backend = TestBackend::new(width, 1);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|f| f.render_widget(Entry(b), f.area()))
                .unwrap();
            terminal.backend().buffer().clone()
        }

        #[test]
        fn key_placed_after_lead() {
            let buf = render_entry(&BINDING, 30);
            assert_eq!(buf.cell(Position::new(LEAD, 0)).unwrap().symbol(), "?");
        }

        #[test]
        fn desc_placed_after_lead_key_gap() {
            let buf = render_entry(&BINDING, 30);
            let x = LEAD + KEY_W + ENTRY_GAP;

            assert_eq!(buf.cell(Position::new(x, 0)).unwrap().symbol(), "z");
            assert_eq!(buf.cell(Position::new(x + 1, 0)).unwrap().symbol(), "i");
            assert_eq!(buf.cell(Position::new(x + 2, 0)).unwrap().symbol(), "g");
        }

        #[test]
        fn long_key_clipped_to_key_width() {
            // Key longer than KEY_W is clipped by the layout rect; desc starts at the correct offset.
            let buf = render_entry(&LONG_KEY, 30);

            assert_eq!(buf.cell(Position::new(LEAD, 0)).unwrap().symbol(), "t");
            assert_eq!(
                buf.cell(Position::new(LEAD + KEY_W + ENTRY_GAP, 0))
                    .unwrap()
                    .symbol(),
                "d"
            );
        }
    }

    mod which_key {
        use super::*;

        mod empty {
            use super::*;

            #[test]
            fn bindings_is_noop() {
                let backend = TestBackend::new(40, 5);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&[], &[]), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer().clone();
                assert!(buf.content.iter().all(|c| c.symbol() == " "));
            }
        }

        mod column_major {
            use super::*;

            static PAIR: [KeyBinding; 2] = [
                KeyBinding {
                    key: "q",
                    desc: "quit",
                },
                KeyBinding {
                    key: "j",
                    desc: "down",
                },
            ];
            static THREE: [KeyBinding; 3] = [
                KeyBinding {
                    key: "a",
                    desc: "alpha",
                },
                KeyBinding {
                    key: "b",
                    desc: "beta",
                },
                KeyBinding {
                    key: "c",
                    desc: "gamma",
                },
            ];
            static FOUR: [KeyBinding; 4] = [
                KeyBinding {
                    key: "a",
                    desc: "alpha",
                },
                KeyBinding {
                    key: "b",
                    desc: "beta",
                },
                KeyBinding {
                    key: "c",
                    desc: "gamma",
                },
                KeyBinding {
                    key: "d",
                    desc: "delta",
                },
            ];

            #[test]
            fn two_bindings_stacked_in_one_column() {
                // area 30×5: 2 bindings, 1 col, 2 rows → popup at y=3..4.
                let backend = TestBackend::new(30, 5);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&PAIR, &[]), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer();

                assert_eq!(buf.cell(Position::new(LEAD, 3)).unwrap().symbol(), "q");
                assert_eq!(buf.cell(Position::new(LEAD, 4)).unwrap().symbol(), "j");
            }

            #[test]
            fn across_two_columns() {
                // area 44×3: cols = ((44+4)/(min_col_w+4)).max(1) = 2, rows = 4/2 = 2.
                // Each Fill(1) column = (44 - COL_GAP) / 2 = min_col_w; col 1 starts at min_col_w + COL_GAP.
                let backend = TestBackend::new(44, 3);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&FOUR, &[]), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer();
                let min_col_w = LEAD + KEY_W + ENTRY_GAP + MIN_DESC_W;
                let col1_key_x = min_col_w + COL_GAP + LEAD;

                assert_eq!(buf.cell(Position::new(LEAD, 1)).unwrap().symbol(), "a");
                assert_eq!(buf.cell(Position::new(LEAD, 2)).unwrap().symbol(), "b");
                assert_eq!(
                    buf.cell(Position::new(col1_key_x, 1)).unwrap().symbol(),
                    "c"
                );
                assert_eq!(
                    buf.cell(Position::new(col1_key_x, 2)).unwrap().symbol(),
                    "d"
                );
            }

            #[test]
            fn partial_last_column_leaves_row_empty() {
                // area 44×3: cols=2, rows=ceil(3/2)=2. col 1 has only 1 entry; its second row is empty.
                let backend = TestBackend::new(44, 3);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&THREE, &[]), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer();
                let min_col_w = LEAD + KEY_W + ENTRY_GAP + MIN_DESC_W;
                let col1_key_x = min_col_w + COL_GAP + LEAD;

                assert_eq!(
                    buf.cell(Position::new(col1_key_x, 1)).unwrap().symbol(),
                    "c"
                );
                assert_eq!(
                    buf.cell(Position::new(col1_key_x, 2)).unwrap().symbol(),
                    " "
                );
            }
        }

        mod merging {
            use super::*;

            static GLOBAL: [KeyBinding; 1] = [KeyBinding {
                key: "g",
                desc: "global",
            }];
            static COMP: [KeyBinding; 1] = [KeyBinding {
                key: "c",
                desc: "comp",
            }];

            #[test]
            fn global_and_component_bindings() {
                // area 30×5: 2 total bindings, 1 col, 2 rows → popup at y=3..4.
                let backend = TestBackend::new(30, 5);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&GLOBAL, &COMP), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer();

                assert_eq!(buf.cell(Position::new(LEAD, 3)).unwrap().symbol(), "g");
                assert_eq!(buf.cell(Position::new(LEAD, 4)).unwrap().symbol(), "c");
            }
        }

        mod height_cap {
            use super::*;

            static FIVE: [KeyBinding; 5] = [
                KeyBinding {
                    key: "a",
                    desc: "one",
                },
                KeyBinding {
                    key: "b",
                    desc: "two",
                },
                KeyBinding {
                    key: "c",
                    desc: "three",
                },
                KeyBinding {
                    key: "d",
                    desc: "four",
                },
                KeyBinding {
                    key: "e",
                    desc: "five",
                },
            ];

            #[test]
            fn bounded_by_area() {
                // 5 bindings in a 30×3 area: rows=5 would exceed area height=3,
                // so popup_h is clamped to 3 and fills the whole terminal.
                let backend = TestBackend::new(30, 3);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&FIVE, &[]), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer();
                assert_eq!(buf.cell(Position::new(LEAD, 0)).unwrap().symbol(), "a");
            }
        }

        mod clipping {
            use super::*;

            static LONG_DESC: [KeyBinding; 1] = [KeyBinding {
                key: "k",
                desc: "averylongdescriptionthatwontfit",
            }];

            #[test]
            fn long_desc_clipped_to_column_width() {
                // Desc longer than the available Fill(1) width is clipped; nothing bleeds past the column.
                let backend = TestBackend::new(30, 3);
                let mut terminal = Terminal::new(backend).unwrap();

                terminal
                    .draw(|f| {
                        f.render_widget(WhichKey::new(&LONG_DESC, &[]), f.area());
                    })
                    .unwrap();

                let buf = terminal.backend().buffer();
                let desc_x = LEAD + KEY_W + ENTRY_GAP;

                assert_eq!(buf.cell(Position::new(desc_x, 2)).unwrap().symbol(), "a");
            }
        }
    }
}
