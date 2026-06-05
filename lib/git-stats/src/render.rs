//! Terminal output rendering with Catppuccin colours.
//!
//! Uses [`anstream`] for ANSI-aware output (auto-strips on non-TTY) and
//! [`catppuccin`] for the colour palette.  The flavour is selected at runtime
//! from [`crate::args::Args::flavour`].
//!
//! ```
//! use dps_git_stats::render::Renderer;
//! use dps_git_stats::args::Flavour;
//!
//! let renderer = Renderer::new(Flavour::Mocha);
//! assert!(format!("{renderer:?}").contains("Renderer"));
//! ```

use std::io::Write;

use anstyle::{AnsiColor, Color, Style};
use polars::prelude::DataFrame;

use crate::args::Flavour;
use crate::error::{Error, StatsError};
use crate::stats::{self, analysis};

/// Bar chart width in characters for the hour distribution.
const BAR_WIDTH: usize = 40;

/// Standard deviation threshold for anomaly detection.
const ANOMALY_SIGMA: f64 = 2.0;

/// Column extractor macro
///
/// Extracts a typed column from a [`DataFrame`] into a `Vec<T>`. The four numeric
/// variants share identical structure; this macro eliminates the repetition.
///
/// Usage:
/// ```text
///   get_col!(df, "name", UInt32, u32, u32)   → Result<Vec<u32>, Error>
///   get_col!(df, "name", Int64,  i64, i64)   → Result<Vec<i64>, Error>
///   get_col!(df, "name", Float64, f64, f64)  → Result<Vec<f64>, Error>
/// ```
macro_rules! get_col {
    ($df:expr, $name:expr, $dtype:ident, $accessor:ident, $T:ty) => {{
        let name: &str = $name;
        $df.column(name)
            .map_err(StatsError::Polars)?
            .cast(&polars::prelude::DataType::$dtype)
            .map_err(StatsError::Polars)?
            .$accessor()
            .map_err(StatsError::Polars)?
            .iter()
            .map(|v| {
                v.ok_or_else(|| Error::Stats(StatsError::MissingColumn(name.to_owned()).into()))
            })
            .collect::<Result<Vec<$T>, Error>>()
    }};
}

/// Drives all terminal output for a single run.
#[derive(Debug)]
pub struct Renderer {
    flavour: catppuccin::Flavor,
}

impl Renderer {
    /// Create a renderer for the given Catppuccin flavour.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps_git_stats::render::Renderer;
    /// use dps_git_stats::args::Flavour;
    ///
    /// let r = Renderer::new(Flavour::Mocha);
    /// let _ = format!("{r:?}");
    /// ```
    #[must_use]
    pub fn new(flavour: Flavour) -> Self {
        Self {
            flavour: catppuccin::Flavor::from(flavour),
        }
    }

    /// Render the full stats table, hour distribution, and analysis footer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Render`] on any IO error, or [`Error::Stats`] on
    /// column access failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps_git_stats::render::Renderer;
    /// use dps_git_stats::args::Flavour;
    /// use dps_git_stats::stats::{build_frame, hour_distribution};
    ///
    /// let df   = build_frame(&[]).unwrap();
    /// let dist = hour_distribution(&[]);
    ///
    /// let mut buf = Vec::new();
    /// Renderer::new(Flavour::Mocha).render(&mut buf, &df, &dist).unwrap();
    /// ```
    pub fn render<W: Write>(
        &self,
        out: &mut W,
        df: &DataFrame,
        dist: &[(u32, u32)],
    ) -> Result<(), Error> {
        if df.height() == 0 {
            return self.render_no_data(out);
        }

        let peak = analysis::peak_hour_band(dist);

        self.render_table(out, df)?;
        self.render_hour_chart(out, dist, peak)?;
        self.render_analysis(out, df, peak)?;

        Ok(())
    }

    /// Render a "no data" message.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Render`] on any IO error.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps_git_stats::render::Renderer;
    /// use dps_git_stats::args::Flavour;
    ///
    /// let mut buf = Vec::new();
    /// Renderer::new(Flavour::Mocha).render_no_data(&mut buf).unwrap();
    /// assert!(!buf.is_empty());
    /// ```
    pub fn render_no_data<W: Write>(&self, out: &mut W) -> Result<(), Error> {
        let style = self.style_overlay();

        write!(
            out,
            "{style}No commits found in the specified range.{style:#}"
        )?;
        writeln!(out)?;

        Ok(())
    }

    fn render_table<W: Write>(&self, out: &mut W, df: &DataFrame) -> Result<(), Error> {
        let header = self.style_header();
        let sep = self.style_separator();
        let pos = self.style_positive();
        let neg = self.style_negative();
        let dim = self.style_dim();

        writeln!(
            out,
            "{header}{:<12} {:>7} {:>8} {:>8} {:>8} {:>6} {:>8} {:>7} {:>8} {:>10} {:>10} {:>10}{header:#}",
            "date",
            "commits",
            "+lines",
            "-lines",
            "total",
            "files",
            "avg_l/c",
            "churn%",
            "roll7",
            "cum+",
            "cum-",
            "cum_total"
        )?;

        writeln!(
            out,
            "{sep}{:<12} {:>7} {:>8} {:>8} {:>8} {:>6} {:>8} {:>7} {:>8} {:>10} {:>10} {:>10}{sep:#}",
            "----",
            "-------",
            "------",
            "------",
            "-----",
            "-----",
            "-------",
            "------",
            "-----",
            "----",
            "----",
            "---------"
        )?;

        let dates = get_str_col(df, "date")?;
        let commits = get_col!(df, "commits", UInt32, u32, u32)?;
        let ins = get_col!(df, "ins", UInt32, u32, u32)?;
        let del = get_col!(df, "del", UInt32, u32, u32)?;
        let total = get_col!(df, "total", Int64, i64, i64)?;
        let files = get_col!(df, "files", UInt32, u32, u32)?;
        let avg_lc = get_col!(df, "avg_lc", Float64, f64, f64)?;
        let churn_pct = get_col!(df, "churn_pct", Float64, f64, f64)?;
        let roll7 = get_col!(df, "roll7", Float64, f64, f64)?;
        let cum_ins = get_col!(df, "cum_ins", Int64, i64, i64)?;
        let cum_del = get_col!(df, "cum_del", Int64, i64, i64)?;
        let cum_total = get_col!(df, "cum_total", Int64, i64, i64)?;

        for i in 0..df.height() {
            let tot = total[i];
            let roll = roll7[i];
            let cum_t = cum_total[i];

            let ts = if tot >= 0 { pos } else { neg };
            let rs = if roll >= 0.0 { pos } else { neg };
            let cs = if cum_t >= 0 { pos } else { neg };

            write!(out, "{dim}{:<12}{dim:#} ", dates[i])?;
            write!(out, "{:>7} {:>8} {:>8} ", commits[i], ins[i], del[i])?;
            write!(out, "{ts}{tot:>+8}{ts:#} ")?;
            write!(
                out,
                "{:>6} {:>8} {:>6}% ",
                files[i], avg_lc[i] as u32, churn_pct[i] as u32
            )?;
            write!(out, "{rs}{roll:>+8.0}{rs:#} ")?;
            write!(out, "{:>10} {:>10} ", cum_ins[i], cum_del[i])?;
            write!(out, "{cs}{cum_t:>+10}{cs:#}")?;

            writeln!(out)?;
        }

        Ok(())
    }

    fn render_hour_chart<W: Write>(
        &self,
        out: &mut W,
        dist: &[(u32, u32)],
        peak: Option<(u32, u32)>,
    ) -> Result<(), Error> {
        let header = self.style_header();

        let slots = stats::hours_to_slots(dist);
        let max_count = slots.iter().copied().max().unwrap_or(1).max(1);

        let bar_style = self.style_bar();
        let peak_style = self.style_positive();
        let dim = self.style_dim();

        writeln!(out)?;
        writeln!(
            out,
            "{header}Commit hour distribution (author local time):{header:#}"
        )?;

        for h in 0u32..24 {
            let count = slots[h as usize];
            let bar_len = (count as usize * BAR_WIDTH) / max_count as usize;
            let bar = "#".repeat(bar_len);
            let in_peak =
                peak.is_some_and(|(start, _)| h >= start && h < start + stats::PEAK_WINDOW);
            let style = if in_peak { peak_style } else { bar_style };

            write!(
                out,
                "  {dim}{h:02}{dim:#}  {style}{bar:<40}{style:#} {dim}{count}{dim:#}"
            )?;

            writeln!(out)?;
        }

        Ok(())
    }

    fn render_analysis<W: Write>(
        &self,
        out: &mut W,
        df: &DataFrame,
        peak: Option<(u32, u32)>,
    ) -> Result<(), Error> {
        let header = self.style_header();
        let pos = self.style_positive();
        let neg = self.style_negative();
        let dim = self.style_dim();

        writeln!(out)?;
        writeln!(out, "{header}Analysis:{header:#}")?;

        if let Some((start, count)) = peak {
            let end = (start + stats::PEAK_WINDOW - 1) % 24;

            writeln!(
                out,
                "  {dim}Peak coding window:{dim:#} \
                 {pos}{start:02}:00–{end:02}:59{pos:#} \
                 ({dim}{count} commits{dim:#})"
            )?;
        }

        match analysis::anomalous_days(df, ANOMALY_SIGMA) {
            Ok(a) if a.height() > 0 => {
                writeln!(
                    out,
                    "  {dim}Anomalous days (>2σ):{dim:#} {neg}{}{neg:#}",
                    a.height()
                )?;
            }
            Ok(_) => writeln!(out, "  {dim}Anomalous days (>2σ):{dim:#} {pos}none{pos:#}")?,
            Err(e) => {
                let mut stderr = anstream::stderr();
                let _ = writeln!(
                    stderr,
                    "  {dim}warning: anomaly analysis skipped: {e}{dim:#}"
                );
            }
        }

        match analysis::weekly_summary(df) {
            Ok(w) if w.height() > 0 => {
                writeln!(
                    out,
                    "  {dim}Active weeks:{dim:#} {pos}{}{pos:#}",
                    w.height()
                )?;
            }
            Ok(_) => {}
            Err(e) => {
                let mut stderr = anstream::stderr();
                let _ = writeln!(stderr, "  {dim}warning: weekly summary skipped: {e}{dim:#}");
            }
        }

        Ok(())
    }

    // colour helpers

    fn style_header(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.lavender).bold()
    }

    fn style_separator(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.overlay0)
    }

    fn style_positive(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.green)
    }

    fn style_negative(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.red)
    }

    fn style_bar(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.blue)
    }

    fn style_dim(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.subtext0)
    }

    fn style_overlay(&self) -> Style {
        catppuccin_fg(&self.flavour.colors.yellow)
    }
}

fn catppuccin_fg(color: &catppuccin::Color) -> Style {
    Style::new().fg_color(Some(Color::Ansi(rgb_to_nearest_ansi(
        color.rgb.r,
        color.rgb.g,
        color.rgb.b,
    ))))
}

/// Map an RGB triple to the nearest ANSI 16-colour code for broad terminal
/// compatibility.
fn rgb_to_nearest_ansi(r: u8, g: u8, b: u8) -> AnsiColor {
    let bright = r.max(g).max(b) > 160;

    match (r > 120, g > 120, b > 120) {
        (true, false, false) => {
            if bright {
                AnsiColor::BrightRed
            } else {
                AnsiColor::Red
            }
        }

        (false, true, false) => {
            if bright {
                AnsiColor::BrightGreen
            } else {
                AnsiColor::Green
            }
        }

        (false, false, true) => {
            if bright {
                AnsiColor::BrightBlue
            } else {
                AnsiColor::Blue
            }
        }

        (true, true, false) => {
            if bright {
                AnsiColor::BrightYellow
            } else {
                AnsiColor::Yellow
            }
        }

        (true, false, true) => {
            if bright {
                AnsiColor::BrightMagenta
            } else {
                AnsiColor::Magenta
            }
        }

        (false, true, true) => {
            if bright {
                AnsiColor::BrightCyan
            } else {
                AnsiColor::Cyan
            }
        }

        (true, true, true) => {
            if bright {
                AnsiColor::BrightWhite
            } else {
                AnsiColor::White
            }
        }

        _ => AnsiColor::BrightBlack,
    }
}

// String column extractor (returns borrowed &str, no cast needed)

fn get_str_col<'a>(df: &'a DataFrame, name: &str) -> Result<Vec<&'a str>, Error> {
    df.column(name)
        .map_err(StatsError::Polars)?
        .str()
        .map_err(StatsError::Polars)?
        .iter()
        .map(|v| v.ok_or_else(|| Error::Stats(StatsError::MissingColumn(name.to_owned()).into())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::args::Flavour;
    use crate::stats::{build_frame, hour_distribution};

    use rstest::rstest;

    mod render {
        use super::*;

        #[rstest]
        fn no_data_writes_message() -> Result<(), &'static str> {
            let mut buf = Vec::new();

            Renderer::new(Flavour::Mocha)
                .render_no_data(&mut buf)
                .map_err(|_| "render_no_data failed")?;

            assert!(String::from_utf8_lossy(&buf).contains("No commits found"));

            Ok(())
        }

        #[rstest]
        fn empty_frame_renders_no_data() -> Result<(), &'static str> {
            let df = build_frame([].as_slice()).map_err(|_| "build_frame")?;
            let dist = hour_distribution([].as_slice());
            let mut buf = Vec::new();

            Renderer::new(Flavour::Mocha)
                .render(&mut buf, &df, &dist)
                .map_err(|_| "render failed")?;

            assert!(String::from_utf8_lossy(&buf).contains("No commits found"));

            Ok(())
        }

        #[rstest]
        #[case(Flavour::Latte)]
        #[case(Flavour::Frappe)]
        #[case(Flavour::Macchiato)]
        #[case(Flavour::Mocha)]
        fn all_flavours_render_no_data(#[case] flavour: Flavour) -> Result<(), &'static str> {
            let mut buf = Vec::new();

            Renderer::new(flavour)
                .render_no_data(&mut buf)
                .map_err(|_| "render_no_data failed")?;

            assert!(!buf.is_empty());

            Ok(())
        }
    }
}
