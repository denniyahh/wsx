//! Spike 002 probe: what gix 0.90 returns for object reads by id, opened the way
//! wsx opens a repository (`src/git/repo.rs` `open_gix`).
//!
//! usage: probe blob <dir> <commit> <name>   -> bytes of <name> in <commit>'s root tree
//!        probe base <dir> <a> <b>           -> merge-base of a and b
use std::error::Error;
use std::io::Write;

use gix::sec::trust::DefaultForLevel;

fn open(start: &str) -> Result<gix::Repository, Box<dyn Error>> {
    let trust_map = gix::sec::trust::Mapping {
        full: gix::open::Options::default_for_level(gix::sec::Trust::Full),
        reduced: gix::open::Options::default_for_level(gix::sec::Trust::Reduced),
    };
    let repo = gix::ThreadSafeRepository::discover_with_environment_overrides_opts(
        start,
        gix::discover::upwards::Options {
            match_ceiling_dir_or_error: false,
            ..Default::default()
        },
        trust_map,
    )?;
    Ok(repo.to_thread_local())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let repo = open(&args[2])?;
    match args[1].as_str() {
        "blob" => {
            let id = gix::ObjectId::from_hex(args[3].as_bytes())?;
            let tree = repo.find_commit(id)?.tree()?;
            let entry = tree.find_entry(args[4].as_str()).ok_or("absent")?;
            let blob = entry.object()?;
            std::io::stdout().write_all(&blob.data)?;
        }
        "base" => {
            let a = gix::ObjectId::from_hex(args[3].as_bytes())?;
            let b = gix::ObjectId::from_hex(args[4].as_bytes())?;
            match repo.merge_base(a, b)? {
                Some(base) => println!("{base}"),
                None => println!("none"),
            }
        }
        other => return Err(format!("unknown mode {other}").into()),
    }
    Ok(())
}
