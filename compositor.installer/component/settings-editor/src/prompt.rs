//! Minimal interactive stdin prompts (no external TUI dependency). Mirrors the
//! installer's `parse.prompt` style: every prompt shows the current value as the
//! `[default]`, and an empty line keeps it. On EOF / non-TTY stdin the default is
//! kept, so `--write-default` and piped runs behave deterministically.

use std::io::{self, Write};

fn read_line() -> Option<String> {
    let mut s = String::new();
    match io::stdin().read_line(&mut s) {
        Ok(0) => None,
        Ok(_) => Some(s.trim().to_string()),
        Err(_) => None,
    }
}

/// Free-text prompt with a default.
pub fn ask(name: &str, desc: &str, default: &str) -> String {
    print!("\n{name}\n  {desc}\n  [{default}] ");
    let _ = io::stdout().flush();
    match read_line() {
        Some(s) if !s.is_empty() => s,
        _ => default.to_string(),
    }
}

/// Yes/no prompt with a default.
pub fn yes_no(name: &str, desc: &str, default: bool) -> bool {
    let d = if default { "Y/n" } else { "y/N" };
    print!("\n{name}\n  {desc}\n  [{d}] ");
    let _ = io::stdout().flush();
    match read_line() {
        Some(s) if !s.is_empty() => matches!(
            s.to_ascii_lowercase().as_str(),
            "y" | "yes" | "1" | "true" | "on"
        ),
        _ => default,
    }
}

/// Choose one of a fixed set of options. Accepts the 1-based index or the literal
/// value; an unrecognized entry keeps the default. `""` renders as `(empty)`.
pub fn choose(name: &str, desc: &str, options: &[&str], default: &str) -> String {
    println!("\n{name}\n  {desc}");
    for (i, opt) in options.iter().enumerate() {
        let shown = if opt.is_empty() { "(empty)" } else { opt };
        let marker = if *opt == default { " *" } else { "" };
        println!("    {}) {}{}", i + 1, shown, marker);
    }
    let def_shown = if default.is_empty() { "(empty)" } else { default };
    print!("  choice [{def_shown}] ");
    let _ = io::stdout().flush();
    match read_line() {
        Some(s) if !s.is_empty() => {
            if let Ok(n) = s.parse::<usize>() {
                if n >= 1 && n <= options.len() {
                    return options[n - 1].to_string();
                }
            }
            if options.iter().any(|o| *o == s) {
                return s;
            }
            println!("  (unrecognized '{s}', keeping default)");
            default.to_string()
        }
        _ => default.to_string(),
    }
}

/// Integer prompt with a default; re-asks on an unparseable entry.
pub fn ask_u8(name: &str, desc: &str, allowed: &[u8], default: u8) -> u8 {
    loop {
        let list: Vec<String> = allowed.iter().map(|v| v.to_string()).collect();
        print!("\n{name}\n  {desc} ({})\n  [{default}] ", list.join("/"));
        let _ = io::stdout().flush();
        match read_line() {
            Some(s) if !s.is_empty() => match s.parse::<u8>() {
                Ok(v) if allowed.contains(&v) => return v,
                _ => println!("  (must be one of {})", list.join("/")),
            },
            _ => return default,
        }
    }
}
