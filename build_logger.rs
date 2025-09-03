// build_logger.rs
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildError {
    pub timestamp: u64,
    pub error_type: String,
    pub message: String,
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub cargo_command: String,
    pub rustc_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildLog {
    pub project_path: PathBuf,
    pub total_builds: u32,
    pub successful_builds: u32,
    pub failed_builds: u32,
    pub errors: Vec<BuildError>,
    pub error_stats: HashMap<String, u32>, // Error type -> count
}

pub struct BuildLogger {
    log_file: PathBuf,
    project_path: PathBuf,
}

impl BuildLogger {
    pub fn new(project_path: impl AsRef<Path>, log_file: impl AsRef<Path>) -> io::Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let log_file = log_file.as_ref().to_path_buf();

        // Create log file if it doesn't exist
        if !log_file.exists() {
            let initial_log = BuildLog {
                project_path: project_path.clone(),
                total_builds: 0,
                successful_builds: 0,
                failed_builds: 0,
                errors: Vec::new(),
                error_stats: HashMap::new(),
            };

            let mut file = File::create(&log_file)?;
            serde_json::to_writer_pretty(&mut file, &initial_log)?;
        }

        Ok(Self {
            log_file,
            project_path,
        })
    }

    pub fn run_build_and_log(&mut self, args: &[&str]) -> io::Result<bool> {
        println!("Running cargo build with args: {:?}", args);

        let rustc_version = Self::get_rustc_version()?;
        let mut command = Command::new("cargo");
        command.args(args);
        command.current_dir(&self.project_path);
        command.stderr(Stdio::piped());

        let output = command.output()?;
        let success = output.status.success();

        // Parse errors from stderr
        let errors = if !success {
            self.parse_cargo_errors(&output.stderr, args, &rustc_version)
        } else {
            Vec::new()
        };

        // Update log
        self.update_log(success, errors)?;

        Ok(success)
    }

    fn get_rustc_version() -> io::Result<String> {
        let output = Command::new("rustc")
            .arg("--version")
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("unknown".to_string())
        }
    }

    fn parse_cargo_errors(
        &self,
        stderr: &[u8],
        args: &[&str],
        rustc_version: &str
    ) -> Vec<BuildError> {
        let mut errors = Vec::new();
        let stderr_str = String::from_utf8_lossy(stderr);
        let cargo_command = args.join(" ");

        for line in stderr_str.lines() {
            if let Some(error) = self.parse_error_line(line, &cargo_command, rustc_version) {
                errors.push(error);
            }
        }

        errors
    }

    fn parse_error_line(
        &self,
        line: &str,
        cargo_command: &str,
        rustc_version: &str
    ) -> Option<BuildError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Parse common Rust error patterns
        if line.contains("error[") {
            return Some(BuildError {
                timestamp,
                error_type: "compiler_error".to_string(),
                message: line.trim().to_string(),
                file_path: self.extract_file_path(line),
                line: self.extract_line_number(line),
                column: self.extract_column_number(line),
                cargo_command: cargo_command.to_string(),
                rustc_version: rustc_version.to_string(),
            });
        }

        if line.contains("error:") && !line.contains("aborting due to") {
            return Some(BuildError {
                timestamp,
                error_type: "general_error".to_string(),
                message: line.trim().to_string(),
                file_path: None,
                line: None,
                column: None,
                cargo_command: cargo_command.to_string(),
                rustc_version: rustc_version.to_string(),
            });
        }

        if line.contains("warning:") {
            return Some(BuildError {
                timestamp,
                error_type: "warning".to_string(),
                message: line.trim().to_string(),
                file_path: self.extract_file_path(line),
                line: self.extract_line_number(line),
                column: self.extract_column_number(line),
                cargo_command: cargo_command.to_string(),
                rustc_version: rustc_version.to_string(),
            });
        }

        None
    }

    fn extract_file_path(&self, line: &str) -> Option<String> {
        // Look for file paths in error messages
        if let Some(start) = line.find("--> ") {
            if let Some(end) = line[start..].find(':') {
                let path = &line[start + 4..start + end];
                return Some(path.trim().to_string());
            }
        }
        None
    }

    fn extract_line_number(&self, line: &str) -> Option<u32> {
        // Extract line numbers from error messages
        if let Some(colon_pos) = line.rfind(':') {
            if let Some(prev_colon) = line[..colon_pos].rfind(':') {
                if let Ok(num) = line[prev_colon + 1..colon_pos].trim().parse() {
                    return Some(num);
                }
            }
        }
        None
    }

    fn extract_column_number(&self, line: &str) -> Option<u32> {
        // Extract column numbers from error messages
        if let Some(colon_pos) = line.rfind(':') {
            if let Ok(num) = line[colon_pos + 1..].trim().parse() {
                return Some(num);
            }
        }
        None
    }

    fn update_log(&mut self, success: bool, new_errors: Vec<BuildError>) -> io::Result<()> {
        let mut log: BuildLog = self.load_log()?;

        log.total_builds += 1;
        if success {
            log.successful_builds += 1;
        } else {
            log.failed_builds += 1;
        }

        for error in new_errors {
            // Update error statistics
            *log.error_stats.entry(error.error_type.clone()).or_insert(0) += 1;
            log.errors.push(error);
        }

        // Keep only the last 1000 errors to prevent log from growing too large
        if log.errors.len() > 1000 {
            log.errors.drain(0..log.errors.len() - 1000);
        }

        self.save_log(&log)
    }

    fn load_log(&self) -> io::Result<BuildLog> {
        let file = File::open(&self.log_file)?;
        let reader = BufReader::new(file);
        let log: BuildLog = serde_json::from_reader(reader)?;
        Ok(log)
    }

    fn save_log(&self, log: &BuildLog) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.log_file)?;

        serde_json::to_writer_pretty(&mut file, log)?;
        Ok(())
    }

    pub fn print_summary(&self) -> io::Result<()> {
        let log = self.load_log()?;

        println!("Build Summary for: {}", self.project_path.display());
        println!("Total builds: {}", log.total_builds);
        println!("Successful: {}", log.successful_builds);
        println!("Failed: {}", log.failed_builds);
        println!("Success rate: {:.1}%",
            (log.successful_builds as f32 / log.total_builds as f32) * 100.0);

        println!("\nError Statistics:");
        for (error_type, count) in &log.error_stats {
            println!("  {}: {}", error_type, count);
        }

        if !log.errors.is_empty() {
            println!("\nRecent Errors:");
            for error in log.errors.iter().rev().take(5) {
                let datetime = DateTime::<Local>::from(UNIX_EPOCH + std::time::Duration::from_secs(error.timestamp));
                println!("[{}] {}: {}", datetime.format("%Y-%m-%d %H:%M:%S"), error.error_type, error.message);
            }
        }

        Ok(())
    }

    pub fn export_log(&self, output_path: impl AsRef<Path>) -> io::Result<()> {
        let log = self.load_log()?;
        let mut file = File::create(output_path)?;
        serde_json::to_writer_pretty(&mut file, &log)?;
        Ok(())
    }
}

// Example usage function
pub fn example_usage() -> io::Result<()> {
    let project_path = "."; // Current directory
    let log_file = "build_log.json";

    let mut logger = BuildLogger::new(project_path, log_file)?;

    // Run a build and log results
    let success = logger.run_build_and_log(&["build"])?;

    if success {
        println!("Build successful!");
    } else {
        println!("Build failed!");
    }

    // Print summary
    logger.print_summary()?;

    // Export log to file
    logger.export_log("build_log_export.json")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_parsing() {
        let logger = BuildLogger::new(".", "test_log.json").unwrap();

        let test_line = "--> src/main.rs:10:5";
        assert_eq!(logger.extract_file_path(test_line), Some("src/main.rs".to_string()));
        assert_eq!(logger.extract_line_number(test_line), Some(10));
        assert_eq!(logger.extract_column_number(test_line), Some(5));
    }
}

// Main function for standalone usage
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <cargo_command> [args...]", args[0]);
        println!("Example: {} build --release", args[0]);
        return Ok(());
    }

    let project_path = ".";
    let log_file = "cargo_build_log.json";

    let mut logger = BuildLogger::new(project_path, log_file)?;

    // Convert args to &[&str] for cargo command
    let cargo_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    let success = logger.run_build_and_log(&cargo_args)?;

    logger.print_summary()?;

    if success {
        println!("\n✅ Build completed successfully");
        std::process::exit(0);
    } else {
        println!("\n❌ Build failed");
        std::process::exit(1);
    }
}