// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides the core abstractions for analyzing executed commands and determining
//! their semantic meaning (e.g., compiler invocations, ignored commands).
//!
//! # Architecture
//!
//! The semantic analysis follows a pipeline approach:
//!
//! 1. **Recognition** ([`Interpreter`] trait) - Analyzes raw [`Execution`] data to identify
//!    known command types (compilers, build tools, etc.)
//!
//! 2. **Classification** ([`Command`] enum) - Represents the recognized command type:
//!    - [`Command::Compiler`] - A compiler invocation with structured arguments
//!    - [`Command::Ignored`] - A command that should be filtered out
//!
//! 3. **Processing** - Further analysis by specialized modules:
//!    - [`clang`] - Converts compiler commands to clang compilation database format
//!    - [`interpreters`] - Various command recognition strategies
//!
//! # Core Types
//!
//! - [`CompilerCommand`] - Represents a structured compiler invocation
//! - [`Arguments`] - Trait for representing different types of compiler arguments
//! - [`ArgumentKind`] - Classifies the semantic meaning of arguments
//! - [`PassEffect`] - Represents how an argument affects the compilation pipeline

pub mod interpreters;

#[cfg(test)]
pub mod testing;

use super::intercept::Execution;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// Implementers of this trait analyze an [`Execution`] and determine if it matches
/// a known command (such as a compiler or interpreter). If recognized, they
/// return a [`Command`] representing the semantic meaning of the execution.
#[cfg_attr(test, mockall::automock)]
pub trait Interpreter: Send + Sync {
    /// An [`Option<Command>`] containing the recognized command, or `None` if not recognized.
    fn recognize(&self, execution: &Execution) -> Option<Command>;
}

/// Represents a recognized command type after semantic analysis.
///
/// This enum aggregates different types of commands that can be recognized
/// by the semantic analysis system.
#[derive(Debug)]
pub enum Command {
    /// A recognized compiler command (e.g., gcc, clang).
    Compiler(CompilerCommand),
    /// A command that is intentionally ignored and not processed further.
    Ignored(&'static str),
}

/// Represents a full compiler command invocation.
///
/// The [`working_dir`] is the directory where the command is executed,
/// the [`executable`] is the path to the compiler binary,
/// the [`resolved_executable`] is the best-effort absolute compiler path resolved
/// using the captured environment (primarily `PATH`),
/// while [`arguments`] contains the command-line arguments annotated
/// with their meaning (e.g., source files, output files, switches).
#[derive(Debug)]
pub struct CompilerCommand {
    pub working_dir: PathBuf,
    pub executable: PathBuf,
    pub resolved_executable: Option<PathBuf>,
    pub arguments: Vec<Box<dyn Arguments>>,
}

/// Trait for representing and converting compiler command arguments.
///
/// This trait provides a unified interface for handling different types of compiler arguments
/// (source files, output files, flags, etc.) and converting them to their string representations
/// for use in compilation databases or command line construction.
pub trait Arguments: std::fmt::Debug {
    /// Returns the semantic kind of this argument group.
    fn kind(&self) -> ArgumentKind;

    /// Converts this argument to its command-line string representation.
    ///
    /// # Parameters
    ///
    /// * `path_updater` - A function that can transform file paths, typically used for
    ///   making paths relative or absolute as needed for the compilation database.
    ///
    /// # Returns
    ///
    /// A vector of strings representing the command-line arguments. For example:
    /// - A source file argument might return `["main.c"]`
    /// - An output flag might return `["-o", "main.o"]`
    /// - A complex flag might return `["-std=c++17"]`
    fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String>;

    /// Extracts a file path from this argument, if applicable.
    ///
    /// This method is used to identify file arguments (typically source or output files)
    /// and extract their paths for use in compilation database entries.
    ///
    /// # Parameters
    ///
    /// * `path_updater` - A function that can transform the extracted path, typically
    ///   used for making paths relative or absolute as needed.
    ///
    /// # Returns
    ///
    /// * `Some(PathBuf)` - If this argument represents a file path
    /// * `None` - If this argument doesn't represent a file (e.g., compiler flags)
    fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf>;
}

/// Represents the meaning of the argument in the compiler call. Identifies
/// the purpose of each argument in the command line.
///
/// Variants:
/// - `Compiler`: The compiler executable itself.
/// - `Source`: A source file to be compiled.
/// - `Output`: An output file or related argument (e.g., `-o output.o`).
/// - `Other`: Any other argument not classified above (e.g., compiler switches like `-Wall`).
///   Specifies how the argument affects the compilation pipeline via [`PassEffect`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArgumentKind {
    Compiler,
    Source,
    Output,
    Other(PassEffect),
}

/// Represents how an argument affects the compilation pipeline.
///
/// This enum distinguishes between arguments that merely configure a compiler pass
/// versus arguments that enforce stopping at a particular pass.
///
/// # Variants
///
/// - `Configures(CompilerPass)`: The argument configures a pass without changing control flow.
///   Examples: `-I/path` (configures preprocessing), `-O2` (configures compilation), `-g` (configures compilation)
///
/// - `StopsAt(CompilerPass)`: The argument enforces stopping at this pass - no later passes run.
///   Examples: `-E` (stops at preprocessing), `-c` (stops at compilation), `-S` (stops at assembly)
///
/// - `InfoAndExit`: The argument causes the compiler to output info and exit immediately.
///   Examples: `--version`, `--help`
///
/// - `DriverOption`: The argument affects driver behavior but doesn't stop or configure a specific pass.
///   Examples: `-pipe`, `-v`, `-save-temps`
///
/// - `None`: The argument's effect on the pipeline is unknown or unclassified.
///   Examples: `-Wall`, `-Wextra`
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PassEffect {
    Configures(CompilerPass),
    StopsAt(CompilerPass),
    InfoAndExit,
    DriverOption,
    None,
}

/// Represents different compiler passes that an argument might affect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompilerPass {
    Preprocessing,
    Compiling,
    Assembling,
    Linking,
}

impl CompilerCommand {
    pub fn new(working_dir: PathBuf, executable: PathBuf, arguments: Vec<Box<dyn Arguments>>) -> Self {
        Self { working_dir, executable, resolved_executable: None, arguments }
    }

    pub fn from_execution(execution: &Execution, arguments: Vec<Box<dyn Arguments>>) -> Self {
        let resolved_executable = resolve_compiler_executable(
            &execution.executable,
            &execution.working_dir,
            &execution.environment,
        );

        Self {
            working_dir: execution.working_dir.clone(),
            executable: execution.executable.clone(),
            resolved_executable,
            arguments,
        }
    }
}

fn resolve_compiler_executable(
    executable: &Path,
    working_dir: &Path,
    environment: &HashMap<String, String>,
) -> Option<PathBuf> {
    // Already an absolute path.
    if executable.is_absolute() {
        return Some(executable.to_path_buf());
    }

    // A path containing separators is not searched in PATH by execvp/spawnp.
    // Resolve it relative to the working directory (best effort).
    if executable.components().count() > 1 {
        let candidate = working_dir.join(executable);
        return candidate.exists().then_some(candidate);
    }

    let path_value = environment
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(crate::environment::KEY_OS__PATH))
        .map(|(_, value)| value.as_str())?;

    which::which_in(executable.as_os_str(), Some(path_value), working_dir).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn resolve_compiler_executable_resolves_short_name_from_path() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let fake_gpp = temp_dir.path().join("g++");
        std::fs::write(&fake_gpp, "").unwrap();
        #[cfg(unix)]
        make_executable(&fake_gpp);

        let mut env = HashMap::new();
        env.insert(
            crate::environment::KEY_OS__PATH.to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        let resolved = resolve_compiler_executable(Path::new("g++"), Path::new("/tmp"), &env)
            .expect("expected a resolved path");
        assert_eq!(resolved, fake_gpp);
    }
}
