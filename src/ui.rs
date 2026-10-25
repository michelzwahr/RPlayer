use std::{io::{self, Stdout, Write, stdout}};

use crossterm::{
    cursor::{self, MoveTo}, execute, queue, style::{Color, Stylize}, terminal::{self, Clear, ClearType},
};
const WIDTH: usize = 52;
const MIN_WIDTH: usize = 40;
const WIDTH_STEP: usize = 2;
const DEFAULT_ACCENT: Color = Color::Green;

const SHUFFLE_SYMBOL: &str = "";
const REPEAT_OFF_SYMBOL: &str = "󰑗";
const REPEAT_ONE_SYMBOL: &str = "󰑘";
const REPEAT_ALL_SYMBOL: &str = "󰑖";
const EXPLICIT_SYMBOL: &str = "󰬌";
const SPOTIFY_SYMBOL: &str = "";
const RUST_ICON: &str = "";

const BAR_FILLED: &str = "";
const BAR_EMPTY: &str = "";
const BAR_FILLED_LEFT: &str = "";
const BAR_FILLED_RIGHT: &str = "";
const BAR_EMPTY_LEFT: &str = "";
const BAR_EMPTY_RIGHT: &str = "";

const PLAY_SYMBOL: &str = "";
const PAUSE_SYMBOL: &str = "";


pub struct UI {
    stdout: Stdout,
    pub width: usize,
    pub fluent_bar: bool,
    pub accent: Color,
}

impl UI {
    pub fn new(width: Option<usize>, fluent: Option<bool>, accent: Option<Color>) -> Self {
        Self {
            stdout: io::stdout(),
            width: width.unwrap_or(WIDTH),
            fluent_bar: fluent.unwrap_or(true),
            accent: accent.unwrap_or(DEFAULT_ACCENT),
        }
    }

    pub fn build(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;

        self.stdout = io::stdout();

        execute!(
            self.stdout,
            Clear(ClearType::All),
            MoveTo(0, 0),
            cursor::Hide,
        )?;

        self.stdout.flush()?;

        Ok(())
    }

    pub fn fit_terminal(&mut self) -> io::Result<()> {
        self.width = terminal::size()?.0 as usize;
        Ok(())
    }

    pub fn stop(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(
            self.stdout,
            Clear(ClearType::All),
            MoveTo(0, 0)
        )?;
        execute!(self.stdout, cursor::Show)?;

        println!("Player beendet.");

        Ok(())
    }

    fn format_time(secs: u64) -> String {
        format!("{}:{:02}", secs / 60, secs % 60)
    }

    fn fit(text: &str, max: usize) -> String {
        if text.chars().count() <= max {
            text.to_string()
        } else {
            let cut: String = text.chars().take(max.saturating_sub(1)).collect();
            format!("{cut}…")
        }
    }

    fn border(&self, y: u16, left: char, right: char) -> io::Result<()> {
        queue!(stdout(), MoveTo(0, y))?;
        print!("{left}{}{right}", "─".repeat(self.width.saturating_sub(2)));
        Ok(())
    }
    
    fn inner_width(&self) -> usize {
        self.width.saturating_sub(4)
    }

    fn row(&self, y: u16, text: &str) -> io::Result<()> {
        queue!(stdout(), MoveTo(0, y))?;
        let inner = self.inner_width();
        print!("│ {:<w$} │", Self::fit(text, inner), w = inner);
        Ok(())
    }

    fn row_styled(&self, y: u16, visible_width: usize, styled: impl std::fmt::Display) -> io::Result<()> {
        queue!(stdout(), MoveTo(0, y))?;
        print!("│ {styled}{} │", " ".repeat(self.inner_width().saturating_sub(visible_width)));
        Ok(())
    }


    fn progress_bar(&self, fluent: bool, elapsed: f64, total: u64) -> String {
    let time_l = Self::format_time(elapsed as u64);
    let time_r = Self::format_time(total);
    let bar_len = self.inner_width().saturating_sub(time_l.len() + time_r.len() + if fluent { 3 } else { 4 });

    // Division durch 0 und Überlauf abfangen
    let ratio = if total == 0 { 0.0 } else { (elapsed / total as f64).clamp(0.0, 1.0) };
    let filled = ratio * bar_len as f64;
    let filled_u = (filled as usize).min(bar_len);

    if fluent {
        const BLOCKS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
        let rest = filled - filled_u as f64;
        let block = if ratio >= 1.0 { '█' } else { BLOCKS[(rest * 8.0).round() as usize] };

        format!(
            "{} {}{}{} {}",
            time_l,
            "█".repeat(filled_u).with(self.accent),
            block.to_string().with(self.accent),
            " ".repeat(bar_len - filled_u),
            time_r
        )
    } else {
        // wie bisher, aber mit filled_u statt eigener Berechnung
        format!(
            "{time_l} {}{}{}{} {time_r}",
            if filled_u != 0 { BAR_FILLED_LEFT } else { BAR_EMPTY_LEFT },
            BAR_FILLED.repeat(filled_u),
            BAR_EMPTY.repeat(bar_len - filled_u),
            if filled_u != bar_len { BAR_EMPTY_RIGHT } else { BAR_FILLED_RIGHT },
        )
    }
}
    
    pub fn draw(&mut self, track_title: String, explicit: bool, track_artists: Vec<String>, album: String, track_duration_secs: u64, elapsed: f64, playing: bool, _frame: u64, shuffle: bool, repeat: u8) -> io::Result<()> {
        let mut y = 0;
        let inner = self.inner_width();
    
        self.border(y, '┌', '┐')?;
        y += 1;
        self.row_styled(y, inner-1, &format!("{} {}{}{}", SPOTIFY_SYMBOL.with(self.accent), "RPlayer".with(self.accent), (" ".repeat(inner-11)), RUST_ICON.with(self.accent)))?;
        y += 1;
        self.border(y, '├', '┤')?;
        y += 1;

        
        let title = Self::fit(&track_title, inner-21);
        let line = format!("{}", title.clone().bold().with(self.accent));
        self.row_styled(y, title.chars().count(), line)?;
        
        y += 1;
        
        let explicit_marker = if explicit { &format!("{EXPLICIT_SYMBOL} ") } else { "" };
        let artists = Self::fit(&(explicit_marker.to_owned() + &track_artists.join(", ")), inner-21);
        let line = format!("{}", artists.clone().with(self.accent).italic().dim());
        self.row_styled(y, artists.chars().count(), line)?;

        y += 1;
        
        let album = Self::fit(&album, inner-21);
        let line = format!("{}", album.clone().grey().dim());
        self.row_styled(y, album.chars().count(), line)?;

        y += 1;
        self.row(y, "")?;
        y += 1;
        self.row(y, "")?;
        y += 1;
        self.row(y, "")?;
        y += 1;
        self.row(y, "")?;
        y += 1;
        self.row(y, "")?;
        y += 1;
        self.border(y, '├', '┤')?;
        y += 1;
        let shuffle_symbol = if shuffle {
            SHUFFLE_SYMBOL.with(self.accent)
        } else {
            SHUFFLE_SYMBOL.grey()
        };
        let repeat_symbol = match repeat {
            1 => REPEAT_ONE_SYMBOL.with(self.accent),
            2 => REPEAT_ALL_SYMBOL.with(self.accent),
            _ => REPEAT_OFF_SYMBOL.grey(),
        };
        let controls_width: usize = 16;
        let controls_distance: usize = controls_width.saturating_sub(2);
        let left_distance: usize = inner.saturating_sub(controls_width).saturating_div(2);
        let total_width = left_distance + controls_distance + 3;

        self.row_styled(
            y,
            total_width,
            format!("{}{shuffle_symbol}{}{}{}{repeat_symbol}", " ".repeat(left_distance), " ".repeat(controls_distance.saturating_div(2)), (if playing { PLAY_SYMBOL } else { PAUSE_SYMBOL }), " ".repeat(controls_distance.saturating_div(2))),
        )?;
        y += 1;
        self.row(y, "")?;
        y += 1;
        self.row_styled(y, self.inner_width(), self.progress_bar(self.fluent_bar, elapsed, track_duration_secs))?;
        y += 1;
        self.border(y, '└', '┘')?;
        y += 1;
    
        queue!(stdout(), MoveTo(0, y + 1))?;
        print!("[←/→] Breite ändern   [q] Beenden");
    
        stdout().flush()
    }

    pub fn update_progress(&mut self, elapsed: f64, total: u64) -> io::Result<()> {
        let y = 14;
        self.row_styled(y, self.inner_width(), self.progress_bar(self.fluent_bar, elapsed, total))?;
        stdout().flush()
    }

    pub fn change_width(&mut self, new_width: usize) {
        self.width = new_width;
        self.clear_right(self.width);
    }

    pub fn increase_width(&mut self) {
        let max_width = terminal::size()
            .map(|(columns, _)| columns as usize)
            .unwrap_or(usize::MAX);
        self.change_width(self.width.saturating_add(WIDTH_STEP).min(max_width));
    }

    pub fn decrease_width(&mut self) {
        self.change_width(self.width.saturating_sub(WIDTH_STEP).max(MIN_WIDTH));
    }

    fn clear_right(&mut self, x: usize) {
        for i in 0..16 {
            execute!(self.stdout, MoveTo(x as u16, i as u16), Clear(ClearType::UntilNewLine)).unwrap();
        }
    }

}