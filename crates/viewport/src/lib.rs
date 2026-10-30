//! Pure cursor-follow crop geometry. No Windows, graphics, authorization or transport.
#![forbid(unsafe_code)]
use std::fmt;

const MAX_SOURCE_PIXELS: u64 = 36_000_000;
const MAX_OUTPUT_PIXELS: u64 = 1920 * 1080;
const MAX_MONITORS: usize = 16;
const FRAME_MS: f64 = 1000.0 / 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    NonFinite,
    InvalidDimensions,
    Overflow,
    InvalidDpi,
    InvalidSettings,
    InvalidTime,
    InvalidTopology,
    DuplicateSource,
    OverlappingSources,
    UnknownSource,
    UnsupportedRotation,
    UnsupportedTransform,
    UnsupportedFullFrame,
    UnrepresentableCrop,
    OutsideSource,
}
impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for GeometryError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        Ok(Self { x, y })
    }
    fn validate(self) -> Result<Self, GeometryError> {
        Self::new(self.x, self.y)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}
impl Size {
    pub fn source(width: u32, height: u32) -> Result<Self, GeometryError> {
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS
        {
            return Err(GeometryError::InvalidDimensions);
        }
        Ok(Self { width, height })
    }
    pub fn output(width: u32, height: u32) -> Result<Self, GeometryError> {
        let size = Self::source(width, height)?;
        if !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
            || u64::from(width) * u64::from(height) > MAX_OUTPUT_PIXELS
        {
            return Err(GeometryError::InvalidDimensions);
        }
        Ok(size)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}
impl PhysicalRect {
    pub fn new(left: i32, top: i32, width: u32, height: u32) -> Result<Self, GeometryError> {
        if width == 0 || height == 0 {
            return Err(GeometryError::InvalidDimensions);
        }
        if i64::from(left) + i64::from(width) > i64::from(i32::MAX) + 1
            || i64::from(top) + i64::from(height) > i64::from(i32::MAX) + 1
        {
            return Err(GeometryError::Overflow);
        }
        Ok(Self {
            left,
            top,
            width,
            height,
        })
    }
    pub fn contains(self, point: Point) -> bool {
        let x = point.x - f64::from(self.left);
        let y = point.y - f64::from(self.top);
        x >= 0.0 && x < f64::from(self.width) && y >= 0.0 && y < f64::from(self.height)
    }
    fn overlaps(self, rhs: Self) -> bool {
        let ax = i64::from(self.left) + i64::from(self.width);
        let ay = i64::from(self.top) + i64::from(self.height);
        let bx = i64::from(rhs.left) + i64::from(rhs.width);
        let by = i64::from(rhs.top) + i64::from(rhs.height);
        i64::from(self.left) < bx
            && i64::from(rhs.left) < ax
            && i64::from(self.top) < by
            && i64::from(rhs.top) < ay
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Unrotated,
    Clockwise90,
    Clockwise180,
    Clockwise270,
    Unknown,
}

/// Host-supplied physical rect, source-local pixel size, topology revision and DPI scale.
/// Identity is a math pin, NEVER a consent grant. Rotation is rejected until verified.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceGeometry {
    id: u64,
    revision: u64,
    desktop: PhysicalRect,
    frame: Size,
    dpi_x: f64,
    dpi_y: f64,
}
impl SourceGeometry {
    pub fn new(
        id: u64,
        revision: u64,
        desktop: PhysicalRect,
        frame: Size,
        dpi_x: f64,
        dpi_y: f64,
        rotation: Rotation,
    ) -> Result<Self, GeometryError> {
        if id == 0 || revision == 0 {
            return Err(GeometryError::InvalidTopology);
        }
        PhysicalRect::new(desktop.left, desktop.top, desktop.width, desktop.height)?;
        let frame = Size::source(frame.width, frame.height)?;
        if !dpi_x.is_finite() || !dpi_y.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if !(0.5..=8.0).contains(&dpi_x) || !(0.5..=8.0).contains(&dpi_y) {
            return Err(GeometryError::InvalidDpi);
        }
        if rotation != Rotation::Unrotated {
            return Err(GeometryError::UnsupportedRotation);
        }
        if u64::from(desktop.width) * u64::from(frame.height)
            != u64::from(desktop.height) * u64::from(frame.width)
        {
            return Err(GeometryError::UnsupportedTransform);
        }
        Ok(Self {
            id,
            revision,
            desktop,
            frame,
            dpi_x,
            dpi_y,
        })
    }
    pub fn id(self) -> u64 {
        self.id
    }
    pub fn revision(self) -> u64 {
        self.revision
    }
    pub fn frame(self) -> Size {
        self.frame
    }
    pub fn desktop(self) -> PhysicalRect {
        self.desktop
    }
    /// ONLY for logical coordinates with explicitly verified local anchors.
    /// Already physical GetCursorPos must NOT be scaled a second time.
    pub fn logical_to_physical(
        self,
        p: Point,
        logical_origin: Point,
        physical_origin: Point,
    ) -> Result<Option<Point>, GeometryError> {
        let (p, l, o) = (
            p.validate()?,
            logical_origin.validate()?,
            physical_origin.validate()?,
        );
        let mapped = Point::new(
            o.x + (p.x - l.x) * self.dpi_x,
            o.y + (p.y - l.y) * self.dpi_y,
        )?;
        Ok(self.desktop.contains(mapped).then_some(mapped))
    }
    pub fn frame_point(self, physical: Point) -> Result<Option<Point>, GeometryError> {
        let physical = physical.validate()?;
        if !self.desktop.contains(physical) {
            return Ok(None);
        }
        Point::new(
            (physical.x - f64::from(self.desktop.left)) * f64::from(self.frame.width)
                / f64::from(self.desktop.width),
            (physical.y - f64::from(self.desktop.top)) * f64::from(self.frame.height)
                / f64::from(self.desktop.height),
        )
        .map(Some)
    }
    pub fn desktop_point(self, frame: Point) -> Result<Option<Point>, GeometryError> {
        let p = frame.validate()?;
        if p.x < 0.0
            || p.y < 0.0
            || p.x >= f64::from(self.frame.width)
            || p.y >= f64::from(self.frame.height)
        {
            return Ok(None);
        }
        let mapped = Point::new(
            f64::from(self.desktop.left)
                + p.x * f64::from(self.desktop.width) / f64::from(self.frame.width),
            f64::from(self.desktop.top)
                + p.y * f64::from(self.desktop.height) / f64::from(self.frame.height),
        )?;
        Ok(self.desktop.contains(mapped).then_some(mapped))
    }
}
/// Source topology is validated for distinct, non-overlapping physical rects.
#[derive(Clone, Debug)]
pub struct Topology {
    sources: Vec<SourceGeometry>,
}
impl Topology {
    pub fn new(sources: Vec<SourceGeometry>) -> Result<Self, GeometryError> {
        if sources.is_empty() || sources.len() > MAX_MONITORS {
            return Err(GeometryError::InvalidTopology);
        }
        let revision = sources[0].revision;
        for (index, source) in sources.iter().enumerate() {
            if source.revision != revision {
                return Err(GeometryError::InvalidTopology);
            }
            for previous in &sources[..index] {
                if source.id == previous.id {
                    return Err(GeometryError::DuplicateSource);
                }
                if source.desktop.overlaps(previous.desktop) {
                    return Err(GeometryError::OverlappingSources);
                }
            }
        }
        Ok(Self { sources })
    }
    pub fn source(&self, id: u64) -> Option<SourceGeometry> {
        self.sources.iter().find(|s| s.id == id).copied()
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    streaming: bool,
    zoom: bool,
    follow: bool,
    magnification: f64,
    smoothing: f64,
}
impl Settings {
    pub fn new(
        streaming: bool,
        zoom: bool,
        follow: bool,
        magnification: f64,
        smoothing: f64,
    ) -> Result<Self, GeometryError> {
        if !magnification.is_finite() || !smoothing.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if !(1.0..=4.0).contains(&magnification) || !(0.0..=1.0).contains(&smoothing) {
            return Err(GeometryError::InvalidSettings);
        }
        Ok(Self {
            streaming,
            zoom,
            follow,
            magnification,
            smoothing,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crop {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}
/// Pin reverse mappings to this exact selected source, topology revision and crop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    pub source: SourceGeometry,
    pub output: Size,
    pub crop: Crop,
}
impl Projection {
    /// Output-local continuous half-open pixels -> selected desktop physical pixels.
    /// Never pass the return value to native input without current host authorization.
    pub fn output_to_physical(self, point: Point) -> Result<Point, GeometryError> {
        let p = point.validate()?;
        if p.x < 0.0
            || p.y < 0.0
            || p.x >= f64::from(self.output.width)
            || p.y >= f64::from(self.output.height)
        {
            return Err(GeometryError::OutsideSource);
        }
        let frame = Point::new(
            f64::from(self.crop.left)
                + p.x * f64::from(self.crop.width) / f64::from(self.output.width),
            f64::from(self.crop.top)
                + p.y * f64::from(self.crop.height) / f64::from(self.output.height),
        )?;
        self.source
            .desktop_point(frame)?
            .ok_or(GeometryError::OutsideSource)
    }
    /// Selected desktop physical cursor -> output pixel if visible within crop.
    pub fn physical_to_output(self, physical: Point) -> Result<Option<Point>, GeometryError> {
        let Some(frame) = self.source.frame_point(physical)? else {
            return Ok(None);
        };
        let x = frame.x - f64::from(self.crop.left);
        let y = frame.y - f64::from(self.crop.top);
        if x < 0.0 || y < 0.0 || x >= f64::from(self.crop.width) || y >= f64::from(self.crop.height)
        {
            return Ok(None);
        }
        Point::new(
            x * f64::from(self.output.width) / f64::from(self.crop.width),
            y * f64::from(self.output.height) / f64::from(self.crop.height),
        )
        .map(Some)
    }
}
#[derive(Clone, Debug, Default)]
pub struct Viewport {
    previous: Option<SourceGeometry>,
    center: Option<Point>,
}
impl Viewport {
    pub fn reset(&mut self) {
        self.previous = None;
        self.center = None;
    }
    /// Time is caller-supplied elapsed monotonic milliseconds, never wall-clock.
    /// Invalid step or streaming OFF is transactional: previous center unchanged.
    pub fn step(
        &mut self,
        topology: &Topology,
        source_id: u64,
        output: Size,
        settings: Settings,
        cursor: Option<Point>,
        dt_ms: f64,
    ) -> Result<Option<Projection>, GeometryError> {
        if !dt_ms.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if !(0.0..=60_000.0).contains(&dt_ms) {
            return Err(GeometryError::InvalidTime);
        }
        let output = Size::output(output.width, output.height)?;
        let settings = Settings::new(
            settings.streaming,
            settings.zoom,
            settings.follow,
            settings.magnification,
            settings.smoothing,
        )?;
        let source = topology
            .source(source_id)
            .ok_or(GeometryError::UnknownSource)?;
        let cursor = match cursor {
            Some(p) => source.frame_point(p)?,
            None => None,
        };
        if !settings.streaming {
            return Ok(None);
        }
        let (width, height) = crop_dimensions(source.frame, output, settings)?;
        let midpoint = Point {
            x: f64::from(source.frame.width) / 2.0,
            y: f64::from(source.frame.height) / 2.0,
        };
        let prior = if self.previous == Some(source) {
            self.center
        } else {
            None
        };
        let initial = prior.unwrap_or(if settings.follow {
            cursor.unwrap_or(midpoint)
        } else {
            midpoint
        });
        let center = if settings.follow {
            if let Some(target) = cursor {
                if prior.is_some() {
                    let alpha = smoothing_alpha(settings.smoothing, dt_ms);
                    Point::new(
                        initial.x + (target.x - initial.x) * alpha,
                        initial.y + (target.y - initial.y) * alpha,
                    )?
                } else {
                    target
                }
            } else {
                initial
            }
        } else {
            initial
        };
        let crop = Crop {
            left: even_origin(center.x, width, source.frame.width),
            top: even_origin(center.y, height, source.frame.height),
            width,
            height,
        };
        self.previous = Some(source);
        self.center = Some(center);
        Ok(Some(Projection {
            source,
            output,
            crop,
        }))
    }
}
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
fn crop_dimensions(
    source: Size,
    output: Size,
    settings: Settings,
) -> Result<(u32, u32), GeometryError> {
    if !settings.zoom {
        if !source.width.is_multiple_of(2)
            || !source.height.is_multiple_of(2)
            || u64::from(source.width) * u64::from(output.height)
                != u64::from(source.height) * u64::from(output.width)
        {
            return Err(GeometryError::UnsupportedFullFrame);
        }
        return Ok((source.width, source.height));
    }
    let divisor = gcd(output.width, output.height);
    let step_w = 2 * (output.width / divisor);
    let step_h = 2 * (output.height / divisor);
    let count_max = (source.width / step_w).min(source.height / step_h);
    let count = (f64::from(count_max) / settings.magnification).floor() as u32;
    if count == 0 {
        return Err(GeometryError::UnrepresentableCrop);
    }
    Ok((step_w * count, step_h * count))
}
fn even_origin(center: f64, width: u32, source: u32) -> u32 {
    let max_units = (source - width) / 2;
    (((center - f64::from(width) / 2.0) / 2.0)
        .round()
        .clamp(0.0, f64::from(max_units)) as u32)
        * 2
}
fn smoothing_alpha(smoothing: f64, dt_ms: f64) -> f64 {
    if smoothing == 1.0 {
        return 1.0;
    }
    if smoothing == 0.0 || dt_ms == 0.0 {
        return 0.0;
    }
    -(((-smoothing).ln_1p() * dt_ms / FRAME_MS).exp_m1())
}
#[cfg(test)]
mod tests;
