//! CLI helper: batch-analyze every mod in a directory and print a Markdown
//! table summarizing client/server support.
//!
//! ```text
//! cargo run -p theseus --example analyze_dir -- "<mods-directory>"
//! ```

use std::path::PathBuf;
use std::process::exit;

use theseus::mod_metadata::mod_analysis::analyze_mod_side_dir;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: analyze_dir <mods-directory>");
        exit(2);
    }

    let dir = PathBuf::from(&args[1]);
    let entries = match analyze_mod_side_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("failed to read {}: {}", dir.display(), err);
            exit(1);
        }
    };

    println!("| Mod file | Type | Name | Version | Server | Client | Environment |");
    println!("| --- | --- | --- | --- | --- | --- | --- |");

    for entry in entries {
        let file_name = entry.path.file_name().unwrap_or_default().to_string_lossy();
        match entry.result {
            Ok(a) => {
                let server = if a.supports_server() { "Yes" } else { "No" };
                let client = if a.supports_client() { "Yes" } else { "No" };
                let name = a.name.as_deref().unwrap_or("-");
                let version = a.version.as_deref().unwrap_or("-");
                println!(
                    "| {} | {:?} | {} | {} | {} | {} | {:?} |",
                    file_name,
                    a.mod_type,
                    name,
                    version,
                    server,
                    client,
                    a.environment()
                );
            }
            Err(err) => {
                println!("| {} | ERROR | - | - | - | - | {} |", file_name, err);
            }
        }
    }
}
