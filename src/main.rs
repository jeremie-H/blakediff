use clap::{command, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use std::collections::{HashMap, HashSet};
use std::fs::{self, DirEntry, File};
use itertools::{self, Itertools};

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
#[command(author, version, about = "🐿️ blakediff - a tool to find duplicates/missing files", long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// read all files in a directory and output hashes for each files with there paths
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
    /// read a report file and display all duplicates hash with paths
    Analyze {
        /// report file to analyze, searching for duplicates
        report_file: String,
    },
    /// compare two report files with hashes and display files present in report_1 and missing in report_2
    Compare {
        /// first report to analyze with
        report_1: String,
        /// second report file
        report_2: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();
    if let Err(e) = match args.command {
        Commands::Generate { dir, report_path, parallel } => generate(dir, report_path, parallel),
        Commands::Compare { report_1, report_2 } => compare(report_1, report_2),
        Commands::Analyze { report_file } => analyze(report_file),
    } {
        panic!("Error {}", e);
    }

    Ok(())
}


fn analyze(report_file: String) -> Result<(), Box<dyn Error>> {
    let input = Input::open(Path::new(&report_file))?;
    let mut buf = io::BufReader::new(input);
    let mut line: String = String::new();
    let mut hmap:HashMap<String, String> = HashMap::new();
    let mut duplicates: HashMap<String, HashSet<String>> = HashMap::new();
    while buf.read_line(&mut line)? != 0 {
        let split = line.split_once(' ').map(|(h,p)| (h.trim(),p.trim()));
        match split {
            Some((hash, path)) => {
                // on est déjà tombé sur ce hash
                if let Some(premier_hash) = hmap.get(hash) {
                    //on a déjà enregistré 2 fichiers ayant ce même hash, on tombe sur un n-ième
                    if let Some(d) = duplicates.get_mut(hash) {
                        d.insert(path.to_owned());
                    }
                    //sinon c'est la première fois qu'on tombe sur un duplica, il faut créer le hashSet
                    else {
                        let mut hs : HashSet<String> = HashSet::new();
                        hs.insert(path.to_owned());
                        hs.insert(premier_hash.to_owned());
                        duplicates.insert(hash.to_owned(), hs);
                        
                    }
                }
                else { // première fois qu'on tombe sur ce hash
                    hmap.insert(hash.to_owned(), path.to_owned());
                    
                }
            },
            None => panic!("issue while compare operation"),
        };
        line.clear();
    }
    //tri d'abord entre les duplicas d'un même fichier (une ligne),
    duplicates.iter()
    .map(|(_hash,set)|set.iter().sorted().collect::<Vec<&String>>())
    .sorted_by_cached_key(|v| v[0]) // puis tri sur les lignes/fichiers (sur le nom du 1er duplica v[0])
    .for_each(|f| {
        print!("duplicates : {}",f.iter().join(" 🟰 "));
        println!();
    });
    Ok(())
}


fn compare(report_1: String, report_2: String) -> Result<(), Box<dyn Error>> {
    let path1 = Path::new(&report_1);
    let path2 = Path::new(&report_2);
    if path1.is_dir() || path2.is_dir() {
        log::error!("Comparison should be avoid on directories, try on report files");
        std::process::exit(1);

    }
    let input1 = File::open(path1)?;
    let input2 = File::open(path2)?;
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

    h1.iter().for_each(|(hash, path)| {
        if !h2.contains_key(hash) {
            println!("only in {} : {}", report_1, path);
        }
    });
    h2.iter().for_each(|(hash, path)| {
        if !h1.contains_key(hash) {
            println!("only in {} : {}", report_2, path);
        }
    });

    analyze(report_1)?;
    analyze(report_2)?;

    Ok(())
}

fn generate(dir: String, _report_path: Option<String>, parallel: bool) -> Result<(), Box<dyn Error>> {
    let took = Timer::new();
    //just display files
    //visit_dirs(Path::new(&args.dir), display_files)?;

    //blake3 on files
    visit_dirs(Path::new(&dir), blake3_mmap, parallel)?;
    
    log::info!("elapsed time : {}", Took::from_std(*took.took().as_std()));
    
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
