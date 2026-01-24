use std::io::{self, Write};

use anyhow::{Result, bail};

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

/// Output handler for consistent formatting
pub struct Output {
    pub format: OutputFormat,
    pub verbose: bool,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            format: OutputFormat::Human,
            verbose: false,
        }
    }
}

impl Output {
    pub fn new(format: OutputFormat, verbose: bool) -> Self {
        Self { format, verbose }
    }

    /// Check that JSON output is not requested for commands that don't support it.
    /// Call this at the start of commands without JSON support.
    pub fn require_human(&self, command: &str) -> Result<()> {
        if self.format == OutputFormat::Json {
            bail!("JSON output is not supported for '{}' command", command);
        }
        Ok(())
    }

    /// Print a status message (action: target)
    pub fn status(&self, action: &str, target: &str) {
        if self.format == OutputFormat::Human {
            // Right-align action in 12 chars, like cargo does
            eprintln!("{:>12} {}", action, target);
        }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if self.format == OutputFormat::Human {
            eprintln!("{:>12} {}", "Done", message);
        }
    }

    /// Print an info message
    pub fn info(&self, message: &str) {
        if self.format == OutputFormat::Human {
            eprintln!("{}", message);
        }
    }

    /// Print a warning message
    pub fn warn(&self, message: &str) {
        if self.format == OutputFormat::Human {
            eprintln!("{:>12} {}", "Warning", message);
        }
    }

    /// Print a verbose message (only if verbose mode is on)
    pub fn verbose(&self, message: &str) {
        if self.verbose && self.format == OutputFormat::Human {
            eprintln!("{}", message);
        }
    }

    /// Flush stderr
    pub fn flush(&self) {
        let _ = io::stderr().flush();
    }
}

/// Print an error message to stderr
pub fn print_error(err: &anyhow::Error) {
    eprintln!("error: {}", err);

    // Print cause chain
    for cause in err.chain().skip(1) {
        eprintln!("  caused by: {}", cause);
    }
}
