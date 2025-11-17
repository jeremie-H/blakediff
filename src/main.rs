use clap::{command, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::Verbosity;
use itertools::{self, Itertools};
use std::collections::{HashMap, HashSet};
use std::fs::{self, DirEntry, File};

use std::io::BufRead;
use std::{
    error::Error,
    io::{self},
    path::Path,
};
use took::{Timer, Took};

use crate::input::Input;
use rayon::prelude::*;
mod input;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output (default)
    Text,
    /// JSON output
    Json,
    /// CSV output
    Csv,
}

/// 🐿️ blakediff - a tool to find duplicates/missing files
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Read all files in a directory and output hashes for each file with their paths
    Generate {
        /// Directory to analyze
        dir: String,

        /// Use multi-threading for walking directories (recommended for SSDs only)
        #[arg(short, long, default_value = "false")]
        parallel: bool,
    },
    /// Read a report file and display all duplicate hashes with paths
    Analyze {
        /// Report file to analyze, searching for duplicates
        report_file: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
    /// Compare two report files and display unique/duplicate files
    Compare {
        /// First report to analyze
        report_1: String,
        /// Second report file
        report_2: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    env_logger::Builder::new().filter_level(args.verbose.log_level_filter()).init();

    match args.command {
        Commands::Generate { dir, parallel } => generate(dir, parallel),
        Commands::Compare { report_1, report_2, format } => compare(report_1, report_2, format),
        Commands::Analyze { report_file, format } => analyze(report_file, format),
    }
}

/// Analyze a report file and find all duplicates
fn analyze(report_file: String, format: OutputFormat) -> Result<(), Box<dyn Error>> {
    let duplicates = find_duplicates_in_report(&report_file)?;

    match format {
        OutputFormat::Text => print_duplicates_text(&duplicates),
        OutputFormat::Json => print_duplicates_json(&duplicates)?,
        OutputFormat::Csv => print_duplicates_csv(&duplicates)?,
    }

    Ok(())
}

/// Parse a report file and find all duplicate hashes
fn find_duplicates_in_report(report_file: &str) -> Result<HashMap<String, HashSet<String>>, Box<dyn Error>> {
    let input = Input::open(Path::new(report_file))?;
    let buf = io::BufReader::new(input);
    let mut hmap: HashMap<String, String> = HashMap::new();
    let mut duplicates: HashMap<String, HashSet<String>> = HashMap::new();

    for (line_num, line) in buf.lines().enumerate() {
        let line = line?;
        let split = line.split_once(' ').map(|(h, p)| (h.trim(), p.trim()));

        match split {
            Some((hash, path)) => {
                // We've already seen this hash
                if let Some(first_path) = hmap.get(hash) {
                    // We've already recorded 2+ files with this hash
                    if let Some(duplicate_set) = duplicates.get_mut(hash) {
                        duplicate_set.insert(path.to_owned());
                    }
                    // First time we found a duplicate for this hash
                    else {
                        let mut hs: HashSet<String> = HashSet::new();
                        hs.insert(path.to_owned());
                        hs.insert(first_path.to_owned());
                        duplicates.insert(hash.to_owned(), hs);
                    }
                } else {
                    // First time seeing this hash
                    hmap.insert(hash.to_owned(), path.to_owned());
                }
            }
            None => {
                return Err(format!("Invalid format at line {}: expected '<hash> <path>', got '{}'", line_num + 1, line).into());
            }
        };
    }

    Ok(duplicates)
}

/// Print duplicates in human-readable text format
fn print_duplicates_text(duplicates: &HashMap<String, HashSet<String>>) {
    // Sort duplicates first by file groups, then by first filename in each group
    duplicates
        .values()
        .map(|set| set.iter().sorted().collect::<Vec<&String>>())
        .sorted_by_cached_key(|v| v[0])
        .for_each(|files| {
            println!("duplicates : {}", files.iter().join(" 🟰 "));
        });
}

/// Print duplicates in JSON format
fn print_duplicates_json(duplicates: &HashMap<String, HashSet<String>>) -> Result<(), Box<dyn Error>> {
    let sorted_duplicates: Vec<Vec<&String>> = duplicates
        .values()
        .map(|set| {
            let mut files: Vec<&String> = set.iter().collect();
            files.sort();
            files
        })
        .sorted_by_cached_key(|v| v[0])
        .collect();

    println!("{{");
    println!("  \"duplicates\": [");
    for (i, files) in sorted_duplicates.iter().enumerate() {
        let files_json: Vec<String> = files.iter().map(|f| format!("\"{}\"", f.replace('\\', "\\\\").replace('"', "\\\""))).collect();
        print!("    [{}]", files_json.join(", "));
        if i < sorted_duplicates.len() - 1 {
            println!(",");
        } else {
            println!();
        }
    }
    println!("  ]");
    println!("}}");

    Ok(())
}

/// Print duplicates in CSV format
fn print_duplicates_csv(duplicates: &HashMap<String, HashSet<String>>) -> Result<(), Box<dyn Error>> {
    println!("hash,file1,file2,file3,...");

    duplicates
        .iter()
        .sorted_by_cached_key(|(_, set)| set.iter().sorted().next().map(|s| s.as_str()))
        .for_each(|(hash, files)| {
            let sorted_files: Vec<&String> = files.iter().sorted().collect();
            let files_csv = sorted_files
                .iter()
                .map(|f| {
                    if f.contains(',') || f.contains('"') || f.contains('\n') {
                        format!("\"{}\"", f.replace('"', "\"\""))
                    } else {
                        (*f).clone()
                    }
                })
                .join(",");
            println!("{},{}", hash, files_csv);
        });

    Ok(())
}

/// Compare two report files and show unique and duplicate files
fn compare(report_1: String, report_2: String, format: OutputFormat) -> Result<(), Box<dyn Error>> {
    let path1 = Path::new(&report_1);
    let path2 = Path::new(&report_2);

    if path1.is_dir() || path2.is_dir() {
        return Err("Comparison should be performed on report files, not directories".into());
    }

    let h1 = parse_report_file(path1)?;
    let h2 = parse_report_file(path2)?;

    let mut only_in_1: Vec<&String> = Vec::new();
    let mut only_in_2: Vec<&String> = Vec::new();
    let mut common: Vec<(&String, &String)> = Vec::new();

    // Find files only in report 1 and common files
    for (hash, path1) in h1.iter() {
        if let Some(path2) = h2.get(hash) {
            common.push((path1, path2));
        } else {
            only_in_1.push(path1);
        }
    }

    // Find files only in report 2
    for (hash, path2) in h2.iter() {
        if !h1.contains_key(hash) {
            only_in_2.push(path2);
        }
    }

    // Sort for consistent output
    only_in_1.sort();
    only_in_2.sort();
    common.sort_by_key(|(p1, _)| *p1);

    match format {
        OutputFormat::Text => {
            print_comparison_text(&report_1, &report_2, &only_in_1, &only_in_2, &common);
        }
        OutputFormat::Json => {
            print_comparison_json(&report_1, &report_2, &only_in_1, &only_in_2, &common)?;
        }
        OutputFormat::Csv => {
            print_comparison_csv(&only_in_1, &only_in_2, &common)?;
        }
    }

    Ok(())
}

/// Parse a report file into a HashMap of hash -> path
fn parse_report_file(path: &Path) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let input = File::open(path)?;
    let buf = io::BufReader::new(input);
    let mut map = HashMap::new();

    for (line_num, line) in buf.lines().enumerate() {
        let line = line?;
        let split = line.split_once(' ').map(|(a, b)| (String::from(a.trim()), String::from(b.trim())));

        match split {
            Some((hash, path)) => {
                map.insert(hash, path);
            }
            None => {
                return Err(format!("Invalid format at line {} in {:?}: expected '<hash> <path>', got '{}'", line_num + 1, path, line).into());
            }
        }
    }

    Ok(map)
}

/// Print comparison results in text format
fn print_comparison_text(report_1: &str, report_2: &str, only_in_1: &[&String], only_in_2: &[&String], common: &[(&String, &String)]) {
    for path in only_in_1 {
        println!("only in {} : {}", report_1, path);
    }

    for path in only_in_2 {
        println!("only in {} : {}", report_2, path);
    }

    for (path1, path2) in common {
        println!("duplicates : {} 🟰 {}", path1, path2);
    }
}

/// Print comparison results in JSON format
fn print_comparison_json(report_1: &str, report_2: &str, only_in_1: &[&String], only_in_2: &[&String], common: &[(&String, &String)]) -> Result<(), Box<dyn Error>> {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");

    println!("{{");
    println!("  \"report_1\": \"{}\",", escape(report_1));
    println!("  \"report_2\": \"{}\",", escape(report_2));

    println!("  \"only_in_report_1\": [");
    for (i, path) in only_in_1.iter().enumerate() {
        print!("    \"{}\"", escape(path));
        if i < only_in_1.len() - 1 {
            println!(",");
        } else {
            println!();
        }
    }
    println!("  ],");

    println!("  \"only_in_report_2\": [");
    for (i, path) in only_in_2.iter().enumerate() {
        print!("    \"{}\"", escape(path));
        if i < only_in_2.len() - 1 {
            println!(",");
        } else {
            println!();
        }
    }
    println!("  ],");

    println!("  \"duplicates\": [");
    for (i, (path1, path2)) in common.iter().enumerate() {
        print!("    {{\"path1\": \"{}\", \"path2\": \"{}\"}}", escape(path1), escape(path2));
        if i < common.len() - 1 {
            println!(",");
        } else {
            println!();
        }
    }
    println!("  ]");
    println!("}}");

    Ok(())
}

/// Print comparison results in CSV format
fn print_comparison_csv(only_in_1: &[&String], only_in_2: &[&String], common: &[(&String, &String)]) -> Result<(), Box<dyn Error>> {
    let csv_escape = |s: &str| {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };

    println!("status,path1,path2");

    for path in only_in_1 {
        println!("only_in_first,{},", csv_escape(path));
    }

    for path in only_in_2 {
        println!("only_in_second,,{}", csv_escape(path));
    }

    for (path1, path2) in common {
        println!("duplicate,{},{}", csv_escape(path1), csv_escape(path2));
    }

    Ok(())
}

/// Generate hash report for all files in a directory
fn generate(dir: String, parallel: bool) -> Result<(), Box<dyn Error>> {
    let took = Timer::new();

    visit_dirs(Path::new(&dir), blake3_mmap, parallel)?;

    log::info!("elapsed time : {}", Took::from_std(*took.took().as_std()));

    Ok(())
}

/// Recursively visit all files in a directory and apply a callback function
fn visit_dirs(dir: &Path, cb: fn(&Path) -> io::Result<()>, parallel: bool) -> Result<(), Box<dyn Error>> {
    if dir.is_dir() {
        let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

        let process_entry = |entry: Result<DirEntry, io::Error>| -> Result<(), String> {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb, parallel).map_err(|e| e.to_string())?;
            } else {
                cb(&path).map_err(|e| format!("Failed to process file {:?}: {}", path, e))?;
            }
            Ok(())
        };

        if parallel {
            let errors: Vec<String> = entries.par_bridge().filter_map(|entry| process_entry(entry).err()).collect();

            if !errors.is_empty() {
                return Err(format!("Multiple errors occurred:\n{}", errors.join("\n")).into());
            }
        } else {
            for entry in entries {
                process_entry(entry).map_err(|e| -> Box<dyn Error> { e.into() })?;
            }
        }
    }

    if dir.is_file() {
        cb(dir)?;
    }

    Ok(())
}

/// Compute BLAKE3 hash for a file and print it
fn blake3_mmap(path: &Path) -> io::Result<()> {
    let mut input = Input::open(path)?;
    let output = input.hash()?;
    println!("{} {}", output, path.to_string_lossy());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn create_temp_report(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("report.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (file_path, temp_dir)
    }

    #[test]
    fn test_parse_report_file() {
        let content = "abc123 /path/to/file1.txt\ndef456 /path/to/file2.txt\n";
        let (file_path, _temp_dir) = create_temp_report(content);

        let result = parse_report_file(&file_path).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("abc123"), Some(&"/path/to/file1.txt".to_string()));
        assert_eq!(result.get("def456"), Some(&"/path/to/file2.txt".to_string()));
    }

    #[test]
    fn test_parse_report_file_invalid_format() {
        let content = "invalid_line_without_space\n";
        let (file_path, _temp_dir) = create_temp_report(content);

        let result = parse_report_file(&file_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid format at line 1"));
    }

    #[test]
    fn test_find_duplicates_in_report_no_duplicates() {
        let content = "abc123 /path/to/file1.txt\ndef456 /path/to/file2.txt\n";
        let (file_path, _temp_dir) = create_temp_report(content);

        let result = find_duplicates_in_report(file_path.to_str().unwrap()).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_find_duplicates_in_report_with_duplicates() {
        let content = "abc123 /path/to/file1.txt\nabc123 /path/to/file2.txt\nabc123 /path/to/file3.txt\n";
        let (file_path, _temp_dir) = create_temp_report(content);

        let result = find_duplicates_in_report(file_path.to_str().unwrap()).unwrap();

        assert_eq!(result.len(), 1);
        let duplicates = result.get("abc123").unwrap();
        assert_eq!(duplicates.len(), 3);
        assert!(duplicates.contains("/path/to/file1.txt"));
        assert!(duplicates.contains("/path/to/file2.txt"));
        assert!(duplicates.contains("/path/to/file3.txt"));
    }

    #[test]
    fn test_find_duplicates_multiple_groups() {
        let content = "abc123 /file1.txt\nabc123 /file2.txt\ndef456 /file3.txt\ndef456 /file4.txt\nghi789 /file5.txt\n";
        let (file_path, _temp_dir) = create_temp_report(content);

        let result = find_duplicates_in_report(file_path.to_str().unwrap()).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("abc123").unwrap().len(), 2);
        assert_eq!(result.get("def456").unwrap().len(), 2);
        assert!(!result.contains_key("ghi789"));
    }

    #[test]
    fn test_compare_files() {
        let content1 = "abc123 /file1.txt\ndef456 /file2.txt\nghi789 /file3.txt\n";
        let content2 = "abc123 /file1_copy.txt\njkl012 /file4.txt\n";

        let (file_path1, _temp_dir1) = create_temp_report(content1);
        let (file_path2, _temp_dir2) = create_temp_report(content2);

        let h1 = parse_report_file(&file_path1).unwrap();
        let h2 = parse_report_file(&file_path2).unwrap();

        let mut only_in_1: Vec<&String> = Vec::new();
        let mut only_in_2: Vec<&String> = Vec::new();
        let mut common: Vec<(&String, &String)> = Vec::new();

        for (hash, path1) in h1.iter() {
            if let Some(path2) = h2.get(hash) {
                common.push((path1, path2));
            } else {
                only_in_1.push(path1);
            }
        }

        for (hash, path2) in h2.iter() {
            if !h1.contains_key(hash) {
                only_in_2.push(path2);
            }
        }

        assert_eq!(only_in_1.len(), 2); // def456 and ghi789
        assert_eq!(only_in_2.len(), 1); // jkl012
        assert_eq!(common.len(), 1); // abc123
    }

    #[test]
    fn test_parse_report_with_spaces_in_path() {
        let content = "abc123 /path/to/file with spaces.txt\n";
        let (file_path, _temp_dir) = create_temp_report(content);

        let result = parse_report_file(&file_path).unwrap();

        assert_eq!(result.get("abc123"), Some(&"/path/to/file with spaces.txt".to_string()));
    }
}
