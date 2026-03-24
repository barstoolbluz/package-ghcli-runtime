use anyhow::{bail, Result};
use std::io::{IsTerminal, Write};

/// Check if a dialoguer error represents Ctrl+C / interrupt.
fn is_interrupted(e: &dialoguer::Error) -> bool {
    match e {
        dialoguer::Error::IO(io_err) => {
            io_err.kind() == std::io::ErrorKind::Interrupted
                || io_err.to_string().to_lowercase().contains("interrupt")
        }
    }
}

/// UI abstraction over dialoguer for interactive prompts.
/// Falls back to non-interactive behavior when stdin is not a TTY
/// or when non_interactive mode is enabled.
pub struct Ui {
    pub non_interactive: bool,
    pub auto_yes: bool,
}

impl Ui {
    pub fn new(non_interactive: bool, auto_yes: bool) -> Self {
        Self {
            non_interactive,
            auto_yes,
        }
    }

    fn is_tty(&self) -> bool {
        std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
    }

    fn interactive(&self) -> bool {
        !self.non_interactive && self.is_tty()
    }

    /// Ask a yes/no confirmation question.
    /// Returns Ok(true) for yes, Ok(false) for no.
    /// In non-interactive mode with auto_yes and default=true, returns Ok(true).
    pub fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        if self.non_interactive {
            if self.auto_yes && default {
                return Ok(true);
            }
            return Ok(false);
        }

        if !self.interactive() {
            return Ok(false);
        }

        let result = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .default(default)
            .interact_opt()?;

        match result {
            Some(v) => Ok(v),
            None => Ok(false), // Ctrl+C / escape
        }
    }

    /// Ask for text input. Returns the trimmed string, or empty string if nothing entered.
    /// Callers should check for "exit"/"quit" themselves if needed.
    pub fn input(&self, prompt: &str, placeholder: &str) -> Result<Option<String>> {
        if !self.interactive() {
            bail!("input required but running non-interactively");
        }

        let display_prompt = if placeholder.is_empty() {
            prompt.to_string()
        } else {
            format!("{} [{}]", prompt, placeholder)
        };

        let theme = dialoguer::theme::ColorfulTheme::default();
        let result = dialoguer::Input::<String>::with_theme(&theme)
            .with_prompt(&display_prompt)
            .allow_empty(true)
            .interact_text();
        match result {
            Ok(v) => Ok(Some(v.trim().to_string())),
            Err(e) if is_interrupted(&e) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Ask for secret/password input with asterisk masking.
    /// Shows `*` for each character typed so the user knows input was received.
    pub fn secret(&self, prompt: &str) -> Result<Option<String>> {
        if !self.interactive() {
            bail!("secret input required but running non-interactively");
        }

        let mut term = console::Term::stderr();
        write!(term, "{} ", dialoguer::theme::ColorfulTheme::default()
            .prompt_prefix)?;
        write!(term, "{}", console::style(prompt).bold())?;
        write!(term, " ")?;
        term.flush()?;

        let mut chars: Vec<char> = Vec::new();
        loop {
            match term.read_key() {
                Ok(console::Key::Enter) => {
                    term.write_line("")?;
                    let value: String = chars.iter().collect();
                    return Ok(Some(value.trim().to_string()));
                }
                Ok(console::Key::Backspace) => {
                    if !chars.is_empty() {
                        chars.pop();
                        term.clear_chars(1)?;
                        term.flush()?;
                    }
                }
                Ok(console::Key::Escape) => {
                    term.write_line("")?;
                    return Ok(None);
                }
                Ok(console::Key::Char(c)) => {
                    chars.push(c);
                    write!(term, "*")?;
                    term.flush()?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    term.write_line("")?;
                    return Ok(None);
                }
                Err(e) => {
                    term.write_line("")?;
                    return Err(e.into());
                }
                _ => {} // ignore other keys
            }
        }
    }

    /// Present a list of options and return the selected one. Returns None if cancelled.
    pub fn select(&self, prompt: &str, options: &[&str]) -> Result<Option<String>> {
        if !self.interactive() {
            bail!("selection required but running non-interactively");
        }

        let result =
            dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .items(options)
                .default(0)
                .interact_opt()?;

        match result {
            Some(idx) => Ok(Some(options[idx].to_string())),
            None => Ok(None),
        }
    }

    /// Present a multi-select checklist and return the indices of selected items.
    pub fn multi_select(&self, prompt: &str, options: &[&str]) -> Result<Vec<usize>> {
        if !self.interactive() {
            bail!("selection required but running non-interactively");
        }

        let result =
            dialoguer::MultiSelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .items(options)
                .interact_opt()?;

        match result {
            Some(indices) => Ok(indices),
            None => Ok(vec![]),
        }
    }

    /// Clear the terminal screen if we're on a TTY.
    pub fn clear(&self) {
        if self.interactive() {
            let term = console::Term::stderr();
            let _ = term.clear_screen();
        }
    }

    /// Print a styled info message.
    pub fn info(&self, msg: &str) {
        eprintln!("{}", msg);
    }

    /// Print a warning.
    pub fn warn(&self, msg: &str) {
        eprintln!("Warning: {}", msg);
    }

    /// Print an error.
    pub fn error(&self, msg: &str) {
        eprintln!("Error: {}", msg);
    }
}
