//! Editing a sprite file's text in place: one value found by its path and
//! rewritten, or put in where it's missing, and every other byte (comments,
//! order, spacing) left as it was. The in-game editor saves through this, so
//! a file stays the same file whether a person, the model or the editor
//! last touched it.
//!
//! ```
//! use platypus_art::edit::{set, Seg};
//! let text = "(\n    palette: {\n        'a': (1, 2, 3), // red\n    },\n)";
//! let text = set(text, &[Seg::Field("palette"), Seg::Char('a')], "(9, 9, 9)").unwrap();
//! assert!(text.contains("'a': (9, 9, 9), // red"));
//! let text = set(&text, &[Seg::Field("anchors"), Seg::Key("hand"), Seg::Key("sit")], "(3, 4)").unwrap();
//! assert!(text.contains("anchors: { \"hand\": { \"sit\": (3, 4) } }"));
//! ```

use std::ops::Range;

/// A step on the way to a value: a struct's field, a map's key (a string or
/// a character), or the n-th item of a list or tuple.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg<'a> {
    Field(&'a str),
    Key(&'a str),
    Char(char),
    Index(usize),
}

impl Seg<'_> {
    fn matches(&self, key: Option<&str>, i: usize) -> bool {
        match *self {
            Seg::Field(n) | Seg::Key(n) => key == Some(n),
            Seg::Char(c) => key.is_some_and(|k| k.chars().eq(std::iter::once(c))),
            Seg::Index(n) => n == i,
        }
    }

    /// As written before a `:`.
    fn written(&self) -> String {
        match *self {
            Seg::Field(n) => n.to_string(),
            Seg::Key(n) => format!("\"{n}\""),
            Seg::Char(c) => format!("'{c}'"),
            Seg::Index(_) => String::new(),
        }
    }
}

#[derive(Debug)]
struct Node {
    span: Range<usize>,
    /// For `(`, `[` and `{`: what's inside.
    group: Option<Vec<Entry>>,
}

#[derive(Debug)]
struct Entry {
    /// `name:` or `"key":` or `'c':` (quotes taken off).
    key: Option<String>,
    value: Node,
    /// Where the entry starts (its key, if it has one) and ends (its comma
    /// included if it has one).
    start: usize,
    end: usize,
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, what: &str) -> String {
        let line = self.s[..self.i.min(self.s.len())].iter().filter(|&&b| b == b'\n').count() + 1;
        format!("line {line}: {what}")
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    /// Past spaces and comments.
    fn skip(&mut self) {
        loop {
            while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
                self.i += 1;
            }
            if self.s[self.i..].starts_with(b"//") {
                while self.peek().is_some_and(|b| b != b'\n') {
                    self.i += 1;
                }
            } else if self.s[self.i..].starts_with(b"/*") {
                self.i += 2;
                while self.i < self.s.len() && !self.s[self.i..].starts_with(b"*/") {
                    self.i += 1;
                }
                self.i = (self.i + 2).min(self.s.len());
            } else {
                return;
            }
        }
    }

    fn quoted(&mut self, q: u8) -> Result<(), String> {
        self.i += 1;
        while let Some(b) = self.peek() {
            self.i += 1;
            if b == b'\\' {
                self.i += 1;
            } else if b == q {
                return Ok(());
            }
        }
        Err(self.err("a quote isn't closed"))
    }

    fn value(&mut self) -> Result<Node, String> {
        self.skip();
        let start = self.i;
        match self.peek().ok_or_else(|| self.err("a value was expected"))? {
            q @ (b'"' | b'\'') => {
                self.quoted(q)?;
                Ok(Node { span: start..self.i, group: None })
            }
            b'(' | b'[' | b'{' => self.group(start),
            _ => {
                while self.peek().is_some_and(|b| !b.is_ascii_whitespace() && !b"(),:[]{}".contains(&b)) {
                    self.i += 1;
                }
                if self.i == start {
                    return Err(self.err("a value was expected"));
                }
                // `Some(...)`, a named struct: the name and what follows.
                let at = self.i;
                self.skip();
                if self.peek() == Some(b'(') {
                    let mut n = self.group(self.i)?;
                    n.span.start = start;
                    return Ok(n);
                }
                self.i = at;
                Ok(Node { span: start..self.i, group: None })
            }
        }
    }

    fn group(&mut self, start: usize) -> Result<Node, String> {
        let close = match self.s[self.i] {
            b'(' => b')',
            b'[' => b']',
            _ => b'}',
        };
        self.i += 1;
        let mut entries = Vec::new();
        loop {
            self.skip();
            match self.peek() {
                None => return Err(self.err("a bracket isn't closed")),
                Some(b) if b == close => {
                    self.i += 1;
                    return Ok(Node { span: start..self.i, group: Some(entries) });
                }
                _ => {}
            }
            let first = self.value()?;
            let start = first.span.start;
            self.skip();
            let (key, value) = if self.peek() == Some(b':') {
                self.i += 1;
                let k = String::from_utf8_lossy(&self.s[first.span.clone()]).into_owned();
                let k = k.trim_matches(|c| c == '"' || c == '\'').to_string();
                (Some(k), self.value()?)
            } else {
                (None, first)
            };
            let mut end = value.span.end;
            self.skip();
            if self.peek() == Some(b',') {
                self.i += 1;
                end = self.i;
            }
            entries.push(Entry { key, value, start, end });
        }
    }
}

fn parse(text: &str) -> Result<Node, String> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    p.value()
}

/// How far a line is indented.
fn indent_of(text: &str, at: usize) -> usize {
    let line = text[..at].rfind('\n').map_or(0, |n| n + 1);
    text[line..].chars().take_while(|c| *c == ' ').count()
}

/// The deepest node along `path`, and how many steps it took.
fn walk<'n>(root: &'n Node, path: &[Seg]) -> (&'n Node, usize) {
    let mut node = root;
    for (depth, seg) in path.iter().enumerate() {
        let Some(e) = node.group.as_ref().and_then(|es| es.iter().enumerate().find(|(i, e)| seg.matches(e.key.as_deref(), *i))) else {
            return (node, depth);
        };
        node = &e.1.value;
    }
    (node, path.len())
}

/// Where the value at `path` is written.
pub fn span(text: &str, path: &[Seg]) -> Result<Range<usize>, String> {
    let root = parse(text)?;
    match walk(&root, path) {
        (n, d) if d == path.len() => Ok(n.span.clone()),
        (_, d) => Err(format!("no {:?}", path[d])),
    }
}

/// The value at `path`, as written.
pub fn get<'t>(text: &'t str, path: &[Seg]) -> Result<&'t str, String> {
    Ok(&text[span(text, path)?])
}

/// Write `value` at `path`: over what's there, or, where the path runs out,
/// as a new entry (maps and structs made for the rest of the way), at the
/// end of the last thing that is there.
pub fn set(text: &str, path: &[Seg], value: &str) -> Result<String, String> {
    let root = parse(text)?;
    let (node, depth) = walk(&root, path);
    let mut out = String::with_capacity(text.len() + value.len() + 16);
    if depth == path.len() {
        out.push_str(&text[..node.span.start]);
        out.push_str(value);
        out.push_str(&text[node.span.end..]);
        return Ok(out);
    }
    let rest = &path[depth..];
    if rest.iter().any(|s| matches!(s, Seg::Index(_))) {
        return Err(format!("no {:?} to write into", rest[0]));
    }
    let entries = node.group.as_ref().ok_or(format!("{:?} isn't a map or a struct", path.get(depth.wrapping_sub(1))))?;
    // The new entry, the rest of the way nested inside it.
    let mut nested = value.to_string();
    for seg in rest[1..].iter().rev() {
        nested = match seg {
            Seg::Field(_) => format!("({}: {nested})", seg.written()),
            _ => format!("{{ {}: {nested} }}", seg.written()),
        };
    }
    let entry = format!("{}: {nested}", rest[0].written());
    let close = node.span.end - 1;
    match entries.last() {
        // After the last entry, on a line of its own like it.
        Some(last) => {
            let multiline = text[node.span.clone()].contains('\n');
            let comma = if last.end == last.value.span.end { "," } else { "" };
            out.push_str(&text[..last.value.span.end]);
            out.push_str(comma);
            out.push_str(&text[last.value.span.end..last.end]);
            if multiline {
                let pad = indent_of(text, last.value.span.start);
                out.push_str(&format!("\n{}{entry},", " ".repeat(pad)));
            } else {
                out.push_str(&format!(" {entry}"));
            }
            out.push_str(&text[last.end..]);
        }
        None => {
            out.push_str(&text[..close]);
            out.push_str(&format!(" {entry} "));
            out.push_str(&text[close..]);
        }
    }
    Ok(out)
}

/// Take the entry at `path` out (with its comma and line, if it has one to
/// itself). Nothing there: the text as it was.
pub fn remove(text: &str, path: &[Seg]) -> Result<String, String> {
    let root = parse(text)?;
    let Some((last, parent_path)) = path.split_last() else { return Err("remove the whole file?".into()) };
    let (parent, depth) = walk(&root, parent_path);
    if depth < parent_path.len() {
        return Ok(text.to_string());
    }
    let Some(e) = parent.group.as_ref().and_then(|es| es.iter().enumerate().find(|(i, e)| last.matches(e.key.as_deref(), *i)).map(|x| x.1)) else {
        return Ok(text.to_string());
    };
    // From the key to the comma; the whole line if it's alone on it.
    let (mut a, mut b) = (e.start, e.end);
    let line_start = text[..a].rfind('\n').map_or(0, |n| n + 1);
    let line_end = text[b..].find('\n').map_or(text.len(), |n| b + n);
    if text[line_start..a].trim().is_empty() && text[b..line_end].trim().is_empty() {
        a = line_start.saturating_sub(1);
        b = line_end;
    }
    Ok(format!("{}{}", &text[..a], &text[b..]))
}

/// Rows of a grid as a list value, one row a line, indented under a line
/// indented `pad`.
pub fn rows_value(rows: &[String], pad: usize) -> String {
    let mut s = String::from("[\n");
    for r in rows {
        s.push_str(&format!("{}\"{r}\",\n", " ".repeat(pad + 4)));
    }
    s.push_str(&" ".repeat(pad));
    s.push(']');
    s
}

/// Replace a grid (a frame's rows, `[Field("frames"), Key(name)]`, or a
/// part's, `[Field("parts"), Key(name), Field("rows")]`), laid out like the
/// file's own.
pub fn set_rows(text: &str, path: &[Seg], rows: &[String]) -> Result<String, String> {
    let at = span(text, path)?;
    set(text, path, &rows_value(rows, indent_of(text, at.start)))
}

/// A grid (a list of rows) at `path`.
pub fn grid(text: &str, path: &[Seg]) -> Result<Vec<String>, String> {
    ron::from_str(get(text, path)?).map_err(|e| format!("{path:?}: not a grid: {e}"))
}

/// A path written with dots: `parts.arm.points.hand`, `palette.'a'`,
/// `poses.stand.0.at` (a quoted `"name"` is a map key; a bare word matches
/// fields and keys alike, but a new entry made from it is a field).
pub fn parse_path(s: &str) -> Vec<Seg<'_>> {
    s.split('.')
        .map(|p| {
            if p.len() >= 3 && p.starts_with('\'') && p.ends_with('\'') {
                Seg::Char(p[1..p.len() - 1].chars().next().unwrap_or('.'))
            } else if p.len() >= 2 && p.starts_with('"') && p.ends_with('"') {
                Seg::Key(&p[1..p.len() - 1])
            } else if let Ok(i) = p.parse() {
                Seg::Index(i)
            } else {
                Seg::Field(p)
            }
        })
        .collect()
}

/// Map keys made when a path runs out: the sections that are maps
/// (`palette`, `frames`, `parts`, `poses`, `anchors`, `clips`, a part's
/// `points`) take quoted keys, the rest fields.
pub fn keyed<'a>(path: &[Seg<'a>]) -> Vec<Seg<'a>> {
    let maps = ["palette", "frames", "derived", "parts", "poses", "anchors", "clips", "points", "fans"];
    let mut out = Vec::with_capacity(path.len());
    for (i, s) in path.iter().enumerate() {
        let after_map = i > 0 && matches!(path[i - 1], Seg::Field(n) if maps.contains(&n)) || (i >= 2 && matches!(path[i - 2], Seg::Field("anchors")));
        out.push(match *s {
            Seg::Field(n) if after_map => Seg::Key(n),
            s => s,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER: &str = include_str!("../../../assets/art/player.ron");

    #[test]
    fn finds_what_it_is_asked_for() {
        assert_eq!(get(PLAYER, &[Seg::Field("size")]).unwrap(), "(24, 25)");
        assert_eq!(get(PLAYER, &[Seg::Field("palette"), Seg::Char('h')]).unwrap(), "(96, 62, 40)");
        assert_eq!(get(PLAYER, &[Seg::Field("parts"), Seg::Key("arm"), Seg::Field("points"), Seg::Key("hand")]).unwrap(), "(0, 5)");
        assert!(get(PLAYER, &[Seg::Field("poses"), Seg::Key("stand"), Seg::Index(0), Seg::Field("at")]).is_ok());
        assert!(get(PLAYER, &[Seg::Field("nothing")]).is_err());
    }

    /// A change touches only itself: the file compiles, the changed part
    /// is changed and every other frame is as it was.
    #[test]
    fn edits_leave_the_rest_alone() {
        let before = crate::compile(&crate::parse(PLAYER).unwrap()).unwrap();
        let path = [Seg::Field("parts"), Seg::Key("head"), Seg::Field("rows")];
        let mut rows: Vec<String> = crate::parse(PLAYER).unwrap().parts["head"].rows.clone();
        rows[0] = rows[0].replacen('h', "k", 1);
        let text = set_rows(PLAYER, &path, &rows).unwrap();
        assert_eq!(text.lines().count(), PLAYER.lines().count(), "laid out as before");
        let file = crate::parse(&text).unwrap();
        assert_eq!(file.parts["head"].rows, rows);
        let after = crate::compile(&file).unwrap();
        let legs = before.index("front_arm@0").unwrap();
        assert_eq!(before.frames[legs], after.frames[legs]);
        assert_ne!(before.frames[before.index("stand").unwrap()], after.frames[after.index("stand").unwrap()]);
        // The comments are still there.
        assert_eq!(text.matches("//").count(), PLAYER.matches("//").count());
    }

    #[test]
    fn paths_read_like_they_are_written() {
        let p = parse_path("anchors.mouth.sit");
        assert_eq!(keyed(&p), vec![Seg::Field("anchors"), Seg::Key("mouth"), Seg::Key("sit")]);
        assert_eq!(keyed(&parse_path("parts.arm.points.hand")), vec![Seg::Field("parts"), Seg::Key("arm"), Seg::Field("points"), Seg::Key("hand")]);
        assert_eq!(parse_path("palette.'a'"), vec![Seg::Field("palette"), Seg::Char('a')]);
        assert_eq!(parse_path("poses.stand.0.at")[2], Seg::Index(0));
        assert_eq!(grid(PLAYER, &parse_path("parts.arm.rows")).unwrap().len(), 6);
    }

    #[test]
    fn missing_things_are_put_in_and_taken_out() {
        let t = set(PLAYER, &[Seg::Field("palette"), Seg::Char('Z')], "(1, 2, 3)").unwrap();
        assert_eq!(crate::parse(&t).unwrap().palette.len(), crate::parse(PLAYER).unwrap().palette.len() + 1);
        let t = set(&t, &[Seg::Field("parts"), Seg::Key("arm"), Seg::Field("points"), Seg::Key("grip")], "(1, 1)").unwrap();
        let t = set(&t, &[Seg::Field("anchors"), Seg::Key("mouth"), Seg::Key("stand")], "(5, 6)").unwrap();
        let f = crate::parse(&t).unwrap();
        assert_eq!(f.parts["arm"].points["grip"], (1, 1));
        assert_eq!(f.anchors["mouth"]["stand"], (5, 6));
        crate::compile(&f).unwrap();
        let t = remove(&t, &[Seg::Field("palette"), Seg::Char('Z')]).unwrap();
        let t = remove(&t, &[Seg::Field("parts"), Seg::Key("arm"), Seg::Field("points"), Seg::Key("grip")]).unwrap();
        let f = crate::parse(&t).unwrap();
        assert!(!f.palette.contains_key(&'Z'));
        assert!(!f.parts["arm"].points.contains_key("grip"));
        assert_eq!(f.parts["arm"].points["hand"], (0, 5));
    }
}
