use clap::{command, Parser, Subcommand};
use std::collections::HashMap;
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

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "blakediff - a tool to find all duplicates files", long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Generate {
        /// directory to analyze
        dir: String,
        /// path where store the report_blakediff.txt
        #[arg(short, long, default_value = ".")]
        report_path: Option<String>,

        /// use multi-threading for walk in directories
        #[arg(short, long, default_value = "false")]
        parallel: bool,
    },
    Compare {
        /// first report to analyze with
        report_1: String,
        /// second report file
        report_2: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if let Err(e) = match args.command {
        Commands::Generate { dir, report_path, parallel } => generate(dir, report_path, parallel),
        Commands::Compare { report_1, report_2 } => compare(report_1, report_2),
    } {
        panic!("Error {}", e);
    }

    Ok(())
}

fn compare(report_1: String, report_2: String) -> Result<(), Box<dyn Error>> {
    let input1 = File::open(Path::new(&report_1))?;
    let input2 = File::open(Path::new(&report_2))?;
    let buf1 = io::BufReader::new(input1);
    let buf2 = io::BufReader::new(input2);

    let h1 = buf1.lines().map(|l| l.unwrap()).fold(HashMap::new(), |mut h, line| {
        let split = line.split_once(' ').map(|(a, b)| (String::from(a), String::from(b)));
        match split {
            Some((hash, path)) => h.insert(hash, path),
            None => panic!("issue while compare operation"),
        };
        h
    });

    let h2 = buf2.lines().map(|l| l.unwrap()).fold(HashMap::new(), |mut h, line| {
        let split = line.split_once(' ').map(|(a, b)| (String::from(a), String::from(b)));
        match split {
            Some((hash, path)) => h.insert(hash, path),
            None => panic!("error occur while comparison"),
        };
        h
    });

    h1.iter().for_each(|entry| {
        if !h2.contains_key(entry.0) {
            println!("not found in {} : {}", report_2, entry.1);
        }
    });

    Ok(())
}

fn generate(dir: String, _report_path: Option<String>, parallel: bool) -> Result<(), Box<dyn Error>> {
    let took = Timer::new();
    //just display files
    //visit_dirs(Path::new(&args.dir), display_files)?;

    //blake3 on files
    visit_dirs(Path::new(&dir), blake3_mmap, parallel)?;

    println!("elapsed time : {}", Took::from_std(*took.took().as_std()));
    Ok(())
}

fn visit_dirs(dir: &Path, cb: fn(&Path) -> io::Result<()>, parallel: bool) -> Result<(), Box<dyn Error>> {
    if dir.is_dir() {
        let it = fs::read_dir(dir).unwrap();
        let parcours = |entry: Result<DirEntry, io::Error>| -> Result<(), Box<dyn Error>> {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb, parallel)?
            } else {
                cb(&path)?
            }
            Ok(())
        };

        if parallel {
            it.par_bridge().for_each(|entry| {
                if let Err(e) = parcours(entry) {
                    panic!("Error {}", e)
                }
            });
        } else {
            it.for_each(|entry| {
                if let Err(e) = parcours(entry) {
                    panic!("Error {}", e)
                }
            });
        }
    }
    if dir.is_file() {
        cb(dir)?;
    }
    Ok(())
}

#[allow(unused)]
fn display_files(path: &Path) -> io::Result<()> {
    println!("Name: {}", path.to_string_lossy());
    Ok(())
}

fn blake3_mmap(path: &Path) -> io::Result<()> {
    let mut input = Input::open(path)?;
    let output = input.hash()?;
    println!("{} {}", output, path.to_string_lossy());
    Ok(())
}
