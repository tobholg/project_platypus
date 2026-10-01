//! `platypus-bestiary [dir] [creature ...]`: the bestiary written out (its
//! cards, live strips and numbers: `bestiary/cli.rs`) for reading without
//! opening the game, by running the game itself headless in the arena. The
//! creatures named, or every one; into `dir` (default `bestiary`):
//! `bestiary.md`, `<id>_card.png`, `<id>_strip.png`.

use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("platypus-bestiary [dir] [creature ...]: every creature's card, live strip and numbers into dir (default ./bestiary)");
        return;
    }
    let dir = args.first().cloned().unwrap_or_else(|| "bestiary".into());
    let dir = std::path::absolute(&dir).unwrap_or_else(|_| dir.clone().into());
    let only = args.iter().skip(1).cloned().collect::<Vec<_>>().join(",");
    let game = std::env::current_exe().ok().and_then(|p| Some(p.parent()?.join("platypus"))).filter(|p| p.exists());
    let Some(game) = game else {
        eprintln!("platypus-bestiary: the game (`platypus`) isn't beside it: build it (cargo build --release -p platypus)");
        exit(2);
    };
    let status = Command::new(game)
        .env("PLATYPUS_BESTIARY", &dir)
        .env("PLATYPUS_BESTIARY_ONLY", only)
        .env("PLATYPUS_WORLD", "arena")
        .env("PLATYPUS_OFFSCREEN", "1")
        .env("PLATYPUS_MUTE", "1")
        .status();
    match status {
        Ok(s) if s.success() => println!("platypus-bestiary: written to {}", dir.display()),
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("platypus-bestiary: {e}");
            exit(2);
        }
    }
}
