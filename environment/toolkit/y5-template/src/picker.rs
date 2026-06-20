//! A small interactive terminal picker with type-to-filter and arrow-key
//! navigation. Used to choose the target L1 directory, since Zed task
//! variables can't expose the project-panel selection.
//!
//! Falls back to a plain numbered prompt when stdin/stdout isn't a TTY.

use std::io::{self, IsTerminal, Write};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, queue, style, terminal};

/// An item shown in the picker: a primary label and an optional dim detail.
/// `selectable=false` makes the item informational (Enter is a no-op).
pub struct Item {
    pub label: String,
    pub detail: String,
    pub selectable: bool,
}

impl Item {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Item { label: label.into(), detail: detail.into(), selectable: true }
    }
    pub fn info(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Item { label: label.into(), detail: detail.into(), selectable: false }
    }
}

/// Show a fuzzy picker. Returns Some((index_in_dynamic_list, query_at_select))
/// or None if cancelled. The dynamic list is rebuilt on each query change by
/// concatenating `extra_for_query(query)` with the fuzzy-filtered base items.
///
/// The caller can detect "user picked a synthetic item I injected" by
/// inspecting `query_at_select` and/or the index range.
pub fn pick(
    prompt: &str,
    base: &[Item],
    extra_for_query: impl Fn(&str) -> Vec<Item>,
) -> io::Result<Option<PickResult>> {
    // Note: we deliberately don't short-circuit. Even with one (or zero)
    // base items, the user may type a query that produces synthetic items,
    // so we always show the picker UI.
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return pick_fallback(prompt, base, &extra_for_query);
    }
    pick_interactive(prompt, base, &extra_for_query)
}

#[derive(Debug, Clone)]
pub struct PickResult {
    /// The query string at the moment of selection.
    pub query: String,
    pub kind: PickKind,
}

#[derive(Debug, Clone)]
pub enum PickKind {
    /// User picked a base item at this index in the original `base` slice.
    Base(usize),
    /// User picked one of the synthetic items returned by `extra_for_query`,
    /// at this index in that returned vector (re-derivable from `query`).
    Extra(usize),
}

/// Subsequence fuzzy match (case-insensitive). Returns true if all chars of
/// `query` appear in order within `hay`.
fn fuzzy(query: &str, hay: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let hay = hay.to_lowercase();
    let mut hc = hay.chars();
    for qc in query.to_lowercase().chars() {
        loop {
            match hc.next() {
                Some(c) if c == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// One row in the rendered display: either an index into `base` or into the
/// freshly-computed extras vector.
#[derive(Clone, Copy)]
enum Row {
    Base(usize),
    Extra(usize),
}

/// Build the combined display list for a given query: extras first, then
/// fuzzy-filtered base.
fn build_rows(base: &[Item], extras: &[Item], query: &str) -> Vec<Row> {
    let mut rows: Vec<Row> = (0..extras.len()).map(Row::Extra).collect();
    for (i, it) in base.iter().enumerate() {
        if fuzzy(query, &it.label) || fuzzy(query, &it.detail) {
            rows.push(Row::Base(i));
        }
    }
    rows
}

fn row_item<'a>(row: Row, base: &'a [Item], extras: &'a [Item]) -> &'a Item {
    match row {
        Row::Base(i) => &base[i],
        Row::Extra(i) => &extras[i],
    }
}

fn pick_interactive(
    prompt: &str,
    base: &[Item],
    extra_for_query: &dyn Fn(&str) -> Vec<Item>,
) -> io::Result<Option<PickResult>> {
    // We render to stderr so stdout stays clean for any machine consumers and
    // so this composes with normal piping.
    let mut out = io::stderr();
    enable_raw_mode()?;

    let result = (|| -> io::Result<Option<PickResult>> {
        let mut query = String::new();
        let mut sel = 0usize; // index into the dynamic row list
        let max_rows = 12usize;

        loop {
            let extras = extra_for_query(&query);
            let rows = build_rows(base, &extras, &query);
            if sel >= rows.len() {
                sel = rows.len().saturating_sub(1);
            }

            // Render: prompt line + up to max_rows entries.
            queue!(out, cursor::MoveToColumn(0))?;
            queue!(out, terminal::Clear(terminal::ClearType::FromCursorDown))?;
            queue!(
                out,
                style::Print(format!("{prompt}  ")),
                style::SetAttribute(style::Attribute::Reverse),
                style::Print(format!(" {query} ")),
                style::SetAttribute(style::Attribute::Reset),
                style::Print(format!("   ({} entr{})\r\n", rows.len(), if rows.len() == 1 { "y" } else { "ies" })),
            )?;

            let shown = rows.len().min(max_rows);
            let start = if sel >= max_rows { sel - max_rows + 1 } else { 0 };
            for (row_i, row) in rows.iter().enumerate().skip(start).take(shown) {
                let it = row_item(*row, base, &extras);
                let is_sel = row_i == sel;
                let dim_unselectable = !it.selectable;
                if is_sel && it.selectable {
                    queue!(
                        out,
                        style::SetForegroundColor(style::Color::Black),
                        style::SetBackgroundColor(style::Color::Cyan),
                        style::Print(format!(" › {}", it.label)),
                        style::SetAttribute(style::Attribute::Reset),
                    )?;
                } else if is_sel {
                    // Selected row but non-selectable — show in reverse-dim.
                    queue!(
                        out,
                        style::SetForegroundColor(style::Color::DarkGrey),
                        style::SetAttribute(style::Attribute::Reverse),
                        style::Print(format!(" › {}", it.label)),
                        style::SetAttribute(style::Attribute::Reset),
                    )?;
                } else if dim_unselectable {
                    queue!(
                        out,
                        style::SetForegroundColor(style::Color::DarkGrey),
                        style::Print(format!("   {}", it.label)),
                        style::SetAttribute(style::Attribute::Reset),
                    )?;
                } else {
                    queue!(out, style::Print(format!("   {}", it.label)))?;
                }
                if !it.detail.is_empty() {
                    queue!(
                        out,
                        style::SetForegroundColor(style::Color::DarkGrey),
                        style::Print(format!("  {}", it.detail)),
                        style::SetAttribute(style::Attribute::Reset),
                    )?;
                }
                queue!(out, style::Print("\r\n"))?;
            }
            queue!(
                out,
                style::SetForegroundColor(style::Color::DarkGrey),
                style::Print("  ↑/↓ or Ctrl-N/P · type to filter · Enter select · Esc cancel"),
                style::SetAttribute(style::Attribute::Reset),
            )?;

            let lines_drawn = 1 + shown + 1;
            queue!(out, cursor::MoveToColumn(0))?;
            if lines_drawn > 0 {
                queue!(out, cursor::MoveToPreviousLine(lines_drawn as u16))?;
            }
            out.flush()?;

            // Read a key.
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match (code, modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    (KeyCode::Enter, _) => {
                        if let Some(row) = rows.get(sel) {
                            let it = row_item(*row, base, &extras);
                            if it.selectable {
                                return Ok(Some(PickResult {
                                    query: query.clone(),
                                    kind: match *row {
                                        Row::Base(i) => PickKind::Base(i),
                                        Row::Extra(i) => PickKind::Extra(i),
                                    },
                                }));
                            }
                            // Non-selectable: ignore Enter, redraw with no change.
                        }
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                        if sel > 0 {
                            sel -= 1;
                        }
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                        if sel + 1 < rows.len() {
                            sel += 1;
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        query.pop();
                        sel = 0;
                    }
                    (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                        query.push(c);
                        sel = 0;
                    }
                    _ => {}
                }
            }
        }
    })();

    let _ = queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::FromCursorDown));
    let _ = out.flush();
    disable_raw_mode()?;
    result
}

/// Non-TTY fallback: read one line. If it contains a slash, treat it as the
/// query for extras and present those + the (unfiltered) base for selection.
/// Otherwise show the base list with optional fuzzy filter applied.
fn pick_fallback(
    prompt: &str,
    base: &[Item],
    extra_for_query: &dyn Fn(&str) -> Vec<Item>,
) -> io::Result<Option<PickResult>> {
    let mut err = io::stderr();
    writeln!(err, "{prompt}")?;
    // In non-TTY mode we can't do live filtering. We accept a single line
    // that's either: a number selecting a base item; OR a string with `/` to
    // trigger the extras path, in which case the FIRST extras item is used.
    for (i, it) in base.iter().enumerate() {
        let mark = if it.selectable { " " } else { "·" };
        if it.detail.is_empty() {
            writeln!(err, "  {mark}{}) {}", i + 1, it.label)?;
        } else {
            writeln!(err, "  {mark}{}) {}  ({})", i + 1, it.label, it.detail)?;
        }
    }
    if base.is_empty() {
        write!(err, "Enter path-spec (containing `/`): ")?;
    } else {
        write!(err, "Enter number (1-{}) or path-spec (containing `/`): ", base.len())?;
    }
    err.flush()?;
    let mut line = String::new();
    if io::stdin().read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let line = line.trim().to_string();
    if line.is_empty() {
        return Ok(None);
    }
    if line.contains('/') {
        let extras = extra_for_query(&line);
        let first_sel = extras.iter().position(|it| it.selectable);
        return Ok(first_sel.map(|i| PickResult {
            query: line,
            kind: PickKind::Extra(i),
        }));
    }
    match line.parse::<usize>() {
        Ok(n) if n >= 1 && n <= base.len() && base[n - 1].selectable => {
            Ok(Some(PickResult { query: String::new(), kind: PickKind::Base(n - 1) }))
        }
        _ => Ok(None),
    }
}
