use std::io::Write;

use anyhow::Result;
use unicode_width::UnicodeWidthStr;

pub fn write_key_values(writer: &mut dyn Write, rows: &[(&str, String)]) -> Result<()> {
    let width = rows
        .iter()
        .map(|(key, _)| UnicodeWidthStr::width(*key))
        .max()
        .unwrap_or(0);

    for (key, value) in rows {
        writeln!(writer, "{key:<width$}  {value}", width = width)?;
    }

    Ok(())
}
