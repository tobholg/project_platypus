//! `platypus-art`: look at and check sprites written as text (assets/art).
//!
//!   platypus-art sheet    <file.ron> [-o out.png] [--scale N] [--no-marks]
//!       a contact sheet: a row per clip, a row of every frame (feet red,
//!       anchors blue); prints which row is which
//!   platypus-art render   <file.ron> <frame> [-o out.png] [--scale N]
//!       one frame
//!   platypus-art check    <file.ron>     errors (exit 1) and warnings
//!   platypus-art describe <file.ron>     the sprite in words
//!   platypus-art get      <file.ron> <path>          a value, as written
//!   platypus-art set      <file.ron> <path> <value>  write it (only if the
//!       file still compiles), everything else left as it was
//!   platypus-art paint    <file.ron> <grid path> x,y=c ...   pixels of a
//!       frame (`frames.sit`) or part (`parts.arm.rows`); '.' clears
//!       paths: `parts.arm.points.hand`, `palette.'a'`, `poses.stand.0.at`
//!   platypus-art import   <picture.png> <frame w> <frame h>
//!       a sprite file's palette and frames from a picture (a sheet of
//!       frames, left to right, top to bottom), printed
//!
//! Default output: next to the file, `<name>.sheet.png` / `<name>.<frame>.png`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use platypus_art::edit::{self, keyed, parse_path};
use platypus_art::{check, compile, describe, parse, read_png, sheet, to_grid, write_png, Pixels};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn load(path: &str) -> Result<(platypus_art::ArtFile, platypus_art::Art), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let file = parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let art = compile(&file).map_err(|e| format!("{path}: {e}"))?;
    Ok((file, art))
}

fn out_path(args: &[String], input: &str, suffix: &str) -> PathBuf {
    flag(args, "-o").map(PathBuf::from).unwrap_or_else(|| {
        let p = Path::new(input);
        p.with_file_name(format!("{}.{suffix}.png", p.file_stem().unwrap_or_default().to_string_lossy()))
    })
}

fn scaled(p: &Pixels, s: u32) -> Pixels {
    let mut out = Pixels::new(p.w * s, p.h * s);
    for y in 0..out.h {
        for x in 0..out.w {
            out.set(x as i32, y as i32, p.get((x / s) as i32, (y / s) as i32));
        }
    }
    out
}

fn run(args: &[String]) -> Result<(), String> {
    let usage = "usage: platypus-art sheet|render|check|describe|get|set|paint|import ... (see the source for details)";
    let cmd = args.first().ok_or(usage)?;
    let input = args.get(1).ok_or(usage)?;
    let scale = flag(args, "--scale").and_then(|s| s.parse().ok()).unwrap_or(8);
    match cmd.as_str() {
        "sheet" => {
            let (_, art) = load(input)?;
            let (pic, legend) = sheet(&art, scale, !args.iter().any(|a| a == "--no-marks"));
            let out = out_path(args, input, "sheet");
            write_png(&pic, &out)?;
            print!("{legend}");
            println!("wrote {}", out.display());
        }
        "render" => {
            let (_, art) = load(input)?;
            let name = args.get(2).ok_or("render <file> <frame>")?;
            let i = art.index(name).ok_or(format!("no frame `{name}`; there are: {}", art.names.join(", ")))?;
            let out = out_path(args, input, name);
            write_png(&scaled(&art.frames[i], scale), &out)?;
            println!("wrote {}", out.display());
        }
        "check" => {
            let (file, art) = load(input)?;
            let warn = check(&file, &art);
            for w in &warn {
                println!("warning: {w}");
            }
            println!("{}: ok ({} warnings)", input, warn.len());
        }
        "describe" => {
            let (file, art) = load(input)?;
            print!("{}", describe(&file, &art));
        }
        "get" => {
            let text = std::fs::read_to_string(input).map_err(|e| format!("{input}: {e}"))?;
            let path = args.get(2).ok_or("get <file> <path>")?;
            println!("{}", edit::get(&text, &keyed(&parse_path(path)))?);
        }
        "set" | "paint" => {
            let text = std::fs::read_to_string(input).map_err(|e| format!("{input}: {e}"))?;
            let path = keyed(&parse_path(args.get(2).ok_or("set <file> <path> <value>")?)).to_vec();
            let new = if cmd == "set" {
                edit::set(&text, &path, args.get(3).ok_or("set <file> <path> <value>")?)?
            } else {
                let mut rows: Vec<Vec<char>> = edit::grid(&text, &path)?.iter().map(|r| r.chars().collect()).collect();
                for px in &args[3..] {
                    let bad = || format!("`{px}`: x,y=c");
                    let (at, c) = px.split_once('=').ok_or_else(bad)?;
                    let (x, y) = at.split_once(',').ok_or_else(bad)?;
                    let (x, y): (usize, usize) = (x.parse().map_err(|_| bad())?, y.parse().map_err(|_| bad())?);
                    let c = c.chars().next().ok_or_else(bad)?;
                    *rows.get_mut(y).and_then(|r| r.get_mut(x)).ok_or(format!("{x},{y} is off the grid"))? = c;
                }
                let rows: Vec<String> = rows.into_iter().map(|r| r.into_iter().collect()).collect();
                edit::set_rows(&text, &path, &rows)?
            };
            let file = parse(&new).map_err(|e| format!("not written, it wouldn't parse: {e}"))?;
            compile(&file).map_err(|e| format!("not written, it wouldn't compile: {e}"))?;
            std::fs::write(input, new).map_err(|e| format!("{input}: {e}"))?;
            println!("wrote {input}");
        }
        "import" => {
            let pic = read_png(Path::new(input))?;
            let fw: u32 = args.get(2).and_then(|s| s.parse().ok()).ok_or("import <png> <frame w> <frame h>")?;
            let fh: u32 = args.get(3).and_then(|s| s.parse().ok()).ok_or("import <png> <frame w> <frame h>")?;
            let mut palette = Vec::new();
            let mut frames = Vec::new();
            for fy in 0..pic.h / fh {
                for fx in 0..pic.w / fw {
                    let rows = to_grid(&pic, fx * fw, fy * fh, fw, fh, &mut palette);
                    if rows.iter().all(|r| r.chars().all(|c| c == '.')) {
                        continue;
                    }
                    frames.push((format!("f{}", frames.len()), rows));
                }
            }
            println!("(\n    size: ({fw}, {fh}),\n    feet: ({}, {fh}),\n    palette: {{", fw / 2);
            for (c, ch) in &palette {
                if c[3] == 255 {
                    println!("        '{ch}': ({}, {}, {}),", c[0], c[1], c[2]);
                } else {
                    println!("        '{ch}': ({}, {}, {}, {}),", c[0], c[1], c[2], c[3]);
                }
            }
            println!("    }},\n    frames: {{");
            for (name, rows) in &frames {
                println!("        \"{name}\": [");
                for r in rows {
                    println!("            \"{r}\",");
                }
                println!("        ],");
            }
            println!("    }},\n)");
        }
        _ => return Err(usage.into()),
    }
    Ok(())
}
