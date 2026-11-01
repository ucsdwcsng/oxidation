//! Diagnostic output helpers.

/// Prints a warning about `file_path` to standard error.
///
/// The output has the form `Warning: <message>: <file_path>`.
pub fn print_warning(file_path: &str, message: &str) {
    eprintln!("Warning: {}: {}", message, file_path);
}
