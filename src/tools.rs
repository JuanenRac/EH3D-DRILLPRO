//! Tool table (`DRILLPRO.DTD`): `nn stroke feed spindle` per line, `.` decimal separator.

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub number: u8,
    /// Additional stroke length in mm.
    pub extra_stroke_mm: f64,
    /// Z feed in mm/min.
    pub feed_mm_min: u32,
    /// Spindle speed in rpm.
    pub spindle_rpm: u32,
}

/// Lines 1-19 are comments; data starts on line 20.
pub fn parse(text: &str) -> Vec<Tool> {
    text.lines()
        .skip(19)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some(Tool {
                number: it.next()?.parse().ok()?,
                extra_stroke_mm: it.next()?.parse().ok()?,
                feed_mm_min: it.next()?.parse().ok()?,
                spindle_rpm: it.next()?.parse().ok()?,
            })
        })
        .collect()
}

/// The text of a tool table file: the 19 comment lines of `original` followed by one line per tool.
pub fn rewrite(original: &str, tools: &[Tool]) -> String {
    let mut out: String = original.split_inclusive('\n').take(19).collect();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for t in tools {
        out.push_str(&format!(
            "{:02} {:.1} {:04} {:05}\r\n",
            t.number, t.extra_stroke_mm, t.feed_mm_min, t.spindle_rpm
        ));
    }
    out
}
