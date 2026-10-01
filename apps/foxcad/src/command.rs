//! Phase 0 interaction model, deliberately independent of egui/winit/wgpu.
//! This will inform the versioned command API in Phase 1, not a document engine.
pub type Point = [f64; 2];
pub type Segment = [Point; 2];

#[derive(Default)]
pub struct LineSession {
    pub anchor: Option<Point>,
    pub first: Option<Point>,
    pub points: usize,
}
#[derive(Default)]
pub struct Commands {
    pub line: Option<LineSession>,
    pub fields: [String; 2],
    pub error: Option<String>,
    pub history: Vec<String>,
    pub segments: Vec<Segment>,
}
impl Commands {
    pub fn invoke(&mut self, name: &str) -> bool {
        if !matches!(name.trim().to_ascii_uppercase().as_str(), "LINE" | "L") {
            self.error = Some(format!("Unknown command: {name}. Try LINE."));
            return false;
        }
        if self.line.is_some() {
            self.finish();
        }
        self.line = Some(LineSession::default());
        self.fields = Default::default();
        self.error = None;
        self.log("LINE — Specify first point".into());
        true
    }
    pub fn prompt(&self) -> &'static str {
        match &self.line {
            Some(line) if line.anchor.is_some() => "Specify next point",
            Some(_) => "Specify first point",
            None => "Enter a command",
        }
    }
    pub fn has_draft(&self) -> bool {
        self.fields.iter().any(|s| !s.trim().is_empty())
    }
    pub fn accept(&mut self, point: Point) {
        let Some(line) = self.line.as_mut() else {
            return;
        };
        if point.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
            self.error = Some("Point must be finite and within ±1e12 drawing units.".into());
            return;
        }
        if let Some(anchor) = line.anchor {
            if anchor == point {
                self.error = Some("Next point must differ from the previous point.".into());
                return;
            }
            self.segments.push([anchor, point]);
        }
        line.anchor = Some(point);
        line.first.get_or_insert(point);
        line.points += 1;
        self.fields = Default::default();
        self.error = None;
        self.log(format!(
            "Point: {:.4}, {:.4} — Specify next point",
            point[0], point[1]
        ));
    }
    pub fn submit(&mut self) {
        if !self.has_draft() {
            if self.line.as_ref().is_some_and(|l| l.anchor.is_some()) {
                self.finish();
            } else {
                self.error = Some("Enter X and Y, or pick a point.".into());
            }
            return;
        }
        let base = self.line.as_ref().and_then(|l| l.anchor);
        match parse_point(&self.fields, base) {
            Ok(point) => self.accept(point),
            Err(message) => self.error = Some(message),
        }
    }
    pub fn escape(&mut self) {
        if self.has_draft() {
            self.fields = Default::default();
            self.error = None;
        } else {
            self.finish();
        }
    }
    pub fn finish(&mut self) {
        if self.line.take().is_some() {
            self.log("LINE finished — accepted segments retained (temporary spike data)".into());
        }
        self.fields = Default::default();
        self.error = None;
    }
    fn log(&mut self, text: String) {
        self.history.push(text);
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }
}

pub fn parse_point(fields: &[String; 2], base: Option<Point>) -> Result<Point, String> {
    let raw = fields[0].trim();
    let relative = raw.starts_with('@');
    let raw = raw.strip_prefix('@').unwrap_or(raw);
    let number = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| "Enter finite numeric coordinates using a decimal point.".to_string())
    };
    let mut point = if let Some((radius, angle)) = raw.split_once('<') {
        if !relative || !fields[1].trim().is_empty() {
            return Err("Use @distance<angle in X, with Y empty.".into());
        }
        let radius = number(radius)?;
        let radians = number(angle)?.to_radians();
        [radius * radians.cos(), radius * radians.sin()]
    } else if let Some((x, y)) = raw.split_once(',') {
        if !fields[1].trim().is_empty() {
            return Err("Use either compact coordinates or separate X/Y fields.".into());
        }
        [number(x)?, number(y)?]
    } else {
        if raw.is_empty() || fields[1].trim().is_empty() {
            return Err("Both X and Y are required.".into());
        }
        [number(raw)?, number(&fields[1])?]
    };
    if relative {
        let origin = base.ok_or("Relative entry requires a previous point.")?;
        for i in 0..2 {
            point[i] += origin[i];
        }
    }
    if point.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
        return Err("Point outside the spike's ±1e12 drawing-unit range.".into());
    }
    Ok(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_and_fields_accept_identical_points() {
        let mut a = Commands::default();
        let mut b = Commands::default();
        a.invoke("LINE");
        b.invoke("l");
        a.fields = ["10,10".into(), "".into()];
        b.fields = ["10".into(), "10".into()];
        a.submit();
        b.submit();
        assert_eq!(a.line.unwrap().anchor, b.line.unwrap().anchor);
    }
    #[test]
    fn partial_enter_and_escape_never_commit() {
        let mut c = Commands::default();
        c.invoke("LINE");
        c.fields[0] = "10".into();
        c.submit();
        assert!(c.error.is_some());
        assert_eq!(c.line.as_ref().unwrap().anchor, None);
        c.escape();
        assert!(c.line.is_some());
        c.escape();
        assert!(c.line.is_none());
        assert!(c.segments.is_empty());
    }
    #[test]
    fn relative_and_polar_entry() {
        assert_eq!(
            parse_point(&["@10,20".into(), "".into()], Some([5.0, 6.0])).unwrap(),
            [15.0, 26.0]
        );
        let p = parse_point(&["@25<90".into(), "".into()], Some([0.0, 0.0])).unwrap();
        assert!(p[0].abs() < 1e-10);
        assert_eq!(p[1], 25.0);
        assert!(parse_point(&["NaN".into(), "0".into()], None).is_err());
    }
    #[test]
    fn finish_retains_accepted_segments() {
        let mut c = Commands::default();
        c.invoke("LINE");
        c.accept([0.0, 0.0]);
        c.accept([10.0, 10.0]);
        c.submit();
        assert!(c.line.is_none());
        assert_eq!(c.segments.len(), 1);
    }
}
