use arboard::Clipboard;
use clap::Parser;
use is_terminal::IsTerminal;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use regex::Regex;
use std::io::{self, Read};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Blur text ranges and regex patterns in terminal output while preserving colors."
)]
struct Args {
    /// Ranges (e.g., 1:1..4:32) or regex patterns to blur
    patterns: Vec<String>,

    /// Preserve spaces in blurred regions
    #[arg(short, long)]
    preserve_spaces: bool,

    /// Hide the line that called the blur command
    #[arg(long)]
    hide_cmd: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pos {
    row: usize, // 1-indexed
    col: usize, // 1-indexed
}

#[derive(Debug, PartialEq, Eq)]
struct Range {
    start: Pos,
    end: Pos,
}

enum Part<'a> {
    Text(&'a str),
    Ansi(&'a str),
}

fn capture_screen() -> Option<String> {
    if let Some(content) = Command::new("tmux")
        .args(["capture-pane", "-e", "-p"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .filter(|c| !c.is_empty())
    {
        return Some(content);
    }

    if let Some(text) = Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok())
        .filter(|t| !t.is_empty())
    {
        return Some(text);
    }

    None
}

fn blur_line(
    line: &str,
    row: usize,
    ranges: &[Range],
    regexes: &[Regex],
    preserve_spaces: bool,
) -> String {
    let ansi_regex = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").unwrap();
    let mut rng = thread_rng();

    // 1. Split line into ANSI parts and Text parts
    let mut parts = Vec::new();
    let mut last_end = 0;
    for m in ansi_regex.find_iter(line) {
        if m.start() > last_end {
            parts.push(Part::Text(&line[last_end..m.start()]));
        }
        parts.push(Part::Ansi(m.as_str()));
        last_end = m.end();
    }
    if last_end < line.len() {
        parts.push(Part::Text(&line[last_end..]));
    }

    // 2. Build visible string for coordinate calculation and regex matching
    let mut visible_text = String::new();
    for part in &parts {
        if let Part::Text(t) = part {
            visible_text.push_str(t);
        }
    }

    let visible_char_count = visible_text.chars().count();
    let mut to_blur = vec![false; visible_char_count];

    // 3. Mark indices from ranges
    for range in ranges {
        if row >= range.start.row && row <= range.end.row {
            let col_start = if row == range.start.row {
                range.start.col
            } else {
                1
            };
            let col_end = if row == range.end.row {
                range.end.col
            } else {
                visible_char_count
            };

            for col in col_start..=col_end {
                if col > 0 && col <= visible_char_count {
                    to_blur[col - 1] = true;
                }
            }
        }
    }

    // 4. Mark indices from regexes
    for re in regexes {
        for m in re.find_iter(&visible_text) {
            let start_char = visible_text[..m.start()].chars().count();
            let end_char = visible_text[..m.end()].chars().count();
            for i in start_char..end_char {
                if i < to_blur.len() {
                    to_blur[i] = true;
                }
            }
        }
    }

    // 5. Reconstruct
    let mut new_line = String::new();
    let mut visible_idx = 0;
    for part in &parts {
        match part {
            Part::Ansi(s) => new_line.push_str(s),
            Part::Text(t) => {
                for c in t.chars() {
                    if visible_idx < to_blur.len() && to_blur[visible_idx] {
                        if preserve_spaces && c == ' ' {
                            new_line.push(' ');
                        } else {
                            new_line.push(rng.sample(Alphanumeric) as char);
                        }
                    } else {
                        new_line.push(c);
                    }
                    visible_idx += 1;
                }
            }
        }
    }
    new_line
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    if args.patterns.is_empty() {
        eprintln!("Usage: blur <pattern1> [pattern2] ...");
        eprintln!("Example: blur 1:33..1:73 4:11..4:27 6:6..6:8");
        std::process::exit(1);
    }

    let mut ranges = Vec::new();
    let mut regexes = Vec::new();
    let range_regex = Regex::new(r"^(\d+):(\d+)\s*\.\.\s*(\d+):(\d+)$").unwrap();

    for pattern in args.patterns {
        if let Some(caps) = range_regex.captures(&pattern) {
            let r1 = caps[1].parse::<usize>().unwrap();
            let c1 = caps[2].parse::<usize>().unwrap();
            let r2 = caps[3].parse::<usize>().unwrap();
            let c2 = caps[4].parse::<usize>().unwrap();
            let p1 = Pos { row: r1, col: c1 };
            let p2 = Pos { row: r2, col: c2 };
            let (start, end) = if p1 <= p2 { (p1, p2) } else { (p2, p1) };
            ranges.push(Range { start, end });
        } else {
            match Regex::new(&pattern) {
                Ok(re) => regexes.push(re),
                Err(e) => {
                    eprintln!("Invalid pattern or regex '{}': {}", pattern, e);
                    std::process::exit(1);
                }
            }
        }
    }

    let input_content = if io::stdin().is_terminal() {
        match capture_screen() {
            Some(content) => content,
            None => {
                eprintln!(
                    "Error: Cannot read terminal buffer. Please run inside tmux or copy your terminal screen to the clipboard first."
                );
                std::process::exit(1);
            }
        }
    } else {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        content
    };

    let ansi_regex = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").unwrap();
    let mut lines: Vec<String> = input_content.lines().map(|s| s.to_string()).collect();

    if args.hide_cmd {
        while let Some(line) = lines.last() {
            let stripped = ansi_regex.replace_all(line, "");
            if stripped.trim().is_empty() {
                lines.pop();
            } else {
                break;
            }
        }
        lines.pop();
    }

    let mut final_lines = Vec::new();
    for (row_idx, line) in lines.iter().enumerate() {
        final_lines.push(blur_line(
            line,
            row_idx + 1,
            &ranges,
            &regexes,
            args.preserve_spaces,
        ));
    }

    if io::stdin().is_terminal() {
        print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    }

    for line in final_lines {
        println!("{}", line);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blur_line_range() {
        let line = "Hello World";
        let ranges = vec![Range {
            start: Pos { row: 1, col: 1 },
            end: Pos { row: 1, col: 5 },
        }];
        let regexes = vec![];
        let blurred = blur_line(line, 1, &ranges, &regexes, false);

        assert_eq!(blurred.len(), 11);
        assert_eq!(&blurred[5..11], " World");
        assert_ne!(&blurred[0..5], "Hello");
    }

    #[test]
    fn test_blur_line_regex() {
        let line = "secret: 12345";
        let ranges = vec![];
        let regexes = vec![Regex::new("12345").unwrap()];
        let blurred = blur_line(line, 1, &ranges, &regexes, false);

        assert_eq!(blurred.len(), 13);
        assert_eq!(&blurred[0..8], "secret: ");
        assert_ne!(&blurred[8..13], "12345");
    }

    #[test]
    fn test_blur_line_ansi_preservation() {
        let red_start = "\x1B[31m";
        let reset = "\x1B[0m";
        let line = format!("{}Red Text{}", red_start, reset);
        let ranges = vec![Range {
            start: Pos { row: 1, col: 1 },
            end: Pos { row: 1, col: 8 },
        }];
        let regexes = vec![];
        let blurred = blur_line(&line, 1, &ranges, &regexes, false);

        assert!(blurred.starts_with(red_start));
        assert!(blurred.ends_with(reset));
        assert_ne!(blurred, line);
        // "Red Text" is 8 characters. ANSI codes should be preserved.
        assert_eq!(blurred.len(), line.len());
    }

    #[test]
    fn test_preserve_spaces() {
        let line = "A B C";
        let ranges = vec![Range {
            start: Pos { row: 1, col: 1 },
            end: Pos { row: 1, col: 5 },
        }];
        let regexes = vec![];

        // With preserve_spaces = true
        let blurred_preserved = blur_line(line, 1, &ranges, &regexes, true);
        assert_eq!(blurred_preserved.chars().nth(1).unwrap(), ' ');
        assert_eq!(blurred_preserved.chars().nth(3).unwrap(), ' ');
        assert_ne!(blurred_preserved, line);

        // With preserve_spaces = false
        let blurred_not_preserved = blur_line(line, 1, &ranges, &regexes, false);
        // It's random, but likely not ' ' (1/62 chance to be ' ' if we used more than Alphanumeric,
        // but Alphanumeric doesn't include space, so it will definitely NOT be space)
        assert_ne!(blurred_not_preserved.chars().nth(1).unwrap(), ' ');
    }
}
