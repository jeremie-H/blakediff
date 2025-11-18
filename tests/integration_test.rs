use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

fn get_binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    if path.ends_with("deps") {
        path.pop(); // Remove deps directory
    }
    path.push("blakediff");
    path
}

#[test]
fn test_generate_command() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").unwrap();

    let output = Command::new(get_binary_path())
        .arg("generate")
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test.txt"), "Output should contain test.txt");
    // Check that output has hash format
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1);
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    assert_eq!(parts.len(), 2, "Output should have format '<hash> <path>'");
}

#[test]
fn test_analyze_command_no_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    let report_file = temp_dir.path().join("report.txt");

    let mut file = fs::File::create(&report_file).unwrap();
    writeln!(file, "abc123 /path/to/file1.txt").unwrap();
    writeln!(file, "def456 /path/to/file2.txt").unwrap();

    let output = Command::new(get_binary_path())
        .arg("analyze")
        .arg(&report_file)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "", "Should have no duplicates");
}

#[test]
fn test_analyze_command_with_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    let report_file = temp_dir.path().join("report.txt");

    let mut file = fs::File::create(&report_file).unwrap();
    writeln!(file, "abc123 /path/to/file1.txt").unwrap();
    writeln!(file, "abc123 /path/to/file2.txt").unwrap();

    let output = Command::new(get_binary_path())
        .arg("analyze")
        .arg(&report_file)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("duplicates"), "Should show duplicates");
    assert!(stdout.contains("file1.txt"), "Should contain file1.txt");
    assert!(stdout.contains("file2.txt"), "Should contain file2.txt");
}

#[test]
fn test_analyze_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let report_file = temp_dir.path().join("report.txt");

    let mut file = fs::File::create(&report_file).unwrap();
    writeln!(file, "abc123 /path/to/file1.txt").unwrap();
    writeln!(file, "abc123 /path/to/file2.txt").unwrap();

    let output = Command::new(get_binary_path())
        .arg("analyze")
        .arg(&report_file)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"duplicates\""), "Should contain JSON with duplicates key");
    assert!(stdout.contains("file1.txt"), "Should contain file1.txt");
    assert!(stdout.contains("file2.txt"), "Should contain file2.txt");
}

#[test]
fn test_compare_command() {
    let temp_dir = TempDir::new().unwrap();
    let report1 = temp_dir.path().join("report1.txt");
    let report2 = temp_dir.path().join("report2.txt");

    let mut file1 = fs::File::create(&report1).unwrap();
    writeln!(file1, "abc123 /path/to/file1.txt").unwrap();
    writeln!(file1, "def456 /path/to/file2.txt").unwrap();

    let mut file2 = fs::File::create(&report2).unwrap();
    writeln!(file2, "abc123 /path/to/file1_copy.txt").unwrap();
    writeln!(file2, "ghi789 /path/to/file3.txt").unwrap();

    let output = Command::new(get_binary_path())
        .arg("compare")
        .arg(&report1)
        .arg(&report2)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file2.txt"), "Should show file2.txt as only in report1");
    assert!(stdout.contains("file3.txt"), "Should show file3.txt as only in report2");
    assert!(stdout.contains("duplicates"), "Should show duplicates for matching hashes");
    assert!(stdout.contains("file1.txt"), "Should contain file1.txt");
    assert!(stdout.contains("file1_copy.txt"), "Should contain file1_copy.txt");
}

#[test]
fn test_compare_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let report1 = temp_dir.path().join("report1.txt");
    let report2 = temp_dir.path().join("report2.txt");

    let mut file1 = fs::File::create(&report1).unwrap();
    writeln!(file1, "abc123 /file1.txt").unwrap();

    let mut file2 = fs::File::create(&report2).unwrap();
    writeln!(file2, "def456 /file2.txt").unwrap();

    let output = Command::new(get_binary_path())
        .arg("compare")
        .arg(&report1)
        .arg(&report2)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"report_1\""), "Should contain report_1 key");
    assert!(stdout.contains("\"report_2\""), "Should contain report_2 key");
    assert!(stdout.contains("\"only_in_report_1\""), "Should contain only_in_report_1 key");
    assert!(stdout.contains("\"only_in_report_2\""), "Should contain only_in_report_2 key");
}

#[test]
fn test_invalid_report_file() {
    let temp_dir = TempDir::new().unwrap();
    let report_file = temp_dir.path().join("invalid.txt");

    let mut file = fs::File::create(&report_file).unwrap();
    writeln!(file, "this_line_has_no_space_separator").unwrap();

    let output = Command::new(get_binary_path())
        .arg("analyze")
        .arg(&report_file)
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "Command should fail with invalid report");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid format") || stderr.contains("Error"), "Should show error about invalid format");
}

#[test]
fn test_generate_with_parallel_flag() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Parallel test").unwrap();

    let output = Command::new(get_binary_path())
        .arg("generate")
        .arg(temp_dir.path())
        .arg("--parallel")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test.txt"), "Output should contain test.txt");
}
