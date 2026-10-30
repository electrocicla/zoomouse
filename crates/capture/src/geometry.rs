//! Pure physical desktop and capture-pixel geometry; no OS operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    Empty,
    Overflow,
    InvalidDpi,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel {
    pub x: u32,
    pub y: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}
impl Size {
    pub fn new(width: u32, height: u32) -> Result<Self, GeometryError> {
        if width == 0 || height == 0 {
            Err(GeometryError::Empty)
        } else {
            Ok(Self { width, height })
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopRect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}
impl DesktopRect {
    pub fn new(left: i32, top: i32, width: u32, height: u32) -> Result<Self, GeometryError> {
        if width == 0 || height == 0 {
            return Err(GeometryError::Empty);
        }
        if i64::from(left) + i64::from(width) > i64::from(i32::MAX)
            || i64::from(top) + i64::from(height) > i64::from(i32::MAX)
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
        let x = i64::from(point.x) - i64::from(self.left);
        let y = i64::from(point.y) - i64::from(self.top);
        x >= 0 && x < i64::from(self.width) && y >= 0 && y < i64::from(self.height)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}
/// Map physical GetCursorPos pixels to source-local frame texels.
/// Requires an explicit verified source rectangle and rotation from the host.
/// Returns None for any cursor outside the source, including half-open edges.
pub fn cursor_to_frame(
    point: Point,
    bounds: DesktopRect,
    frame: Size,
    rotation: Rotation,
) -> Option<Pixel> {
    if !bounds.contains(point) || frame.width == 0 || frame.height == 0 {
        return None;
    }
    let x = (i64::from(point.x) - i64::from(bounds.left)) as f64 / f64::from(bounds.width);
    let y = (i64::from(point.y) - i64::from(bounds.top)) as f64 / f64::from(bounds.height);
    let (u, v) = match rotation {
        Rotation::None => (x, y),
        Rotation::Clockwise90 => (1.0 - y, x),
        Rotation::Clockwise180 => (1.0 - x, 1.0 - y),
        Rotation::Clockwise270 => (y, 1.0 - x),
    };
    Some(Pixel {
        x: ((u * f64::from(frame.width)).floor() as u32).min(frame.width - 1),
        y: ((v * f64::from(frame.height)).floor() as u32).min(frame.height - 1),
    })
}
/// Only for logical window coordinates, NOT physical PerMonitorV2 GetCursorPos.
#[derive(Debug, Clone, Copy)]
pub struct DpiMapping {
    pub logical_origin: Point,
    pub physical_origin: Point,
    pub dpi_x: u32,
    pub dpi_y: u32,
}
impl DpiMapping {
    pub fn to_physical(self, p: Point) -> Result<Point, GeometryError> {
        if !(48..=768).contains(&self.dpi_x) || !(48..=768).contains(&self.dpi_y) {
            return Err(GeometryError::InvalidDpi);
        }
        fn axis(pos: i32, from: i32, to: i32, dpi: u32) -> Result<i32, GeometryError> {
            let delta = i64::from(pos) - i64::from(from);
            let value = i64::from(to)
                + delta
                    .checked_mul(i64::from(dpi))
                    .ok_or(GeometryError::Overflow)?
                    .div_euclid(96);
            i32::try_from(value).map_err(|_| GeometryError::Overflow)
        }
        Ok(Point {
            x: axis(
                p.x,
                self.logical_origin.x,
                self.physical_origin.x,
                self.dpi_x,
            )?,
            y: axis(
                p.y,
                self.logical_origin.y,
                self.physical_origin.y,
                self.dpi_y,
            )?,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_origin_edges() {
        let r = DesktopRect::new(-1920, -1080, 1920, 1080).unwrap();
        let s = Size::new(960, 540).unwrap();
        assert_eq!(
            cursor_to_frame(Point { x: -1920, y: -1080 }, r, s, Rotation::None),
            Some(Pixel { x: 0, y: 0 })
        );
        assert_eq!(
            cursor_to_frame(Point { x: -1, y: -1 }, r, s, Rotation::None),
            Some(Pixel { x: 959, y: 539 })
        );
        assert_eq!(
            cursor_to_frame(Point { x: 0, y: 0 }, r, s, Rotation::None),
            None
        );
        assert_eq!(
            cursor_to_frame(Point { x: -1921, y: -1080 }, r, s, Rotation::None),
            None
        );
    }
    #[test]
    fn rotation_without_screen_assumptions() {
        let r = DesktopRect::new(100, 200, 100, 50).unwrap();
        let s = Size::new(50, 100).unwrap();
        let p = Point { x: 125, y: 210 };
        assert_eq!(
            cursor_to_frame(p, r, s, Rotation::Clockwise90),
            Some(Pixel { x: 40, y: 25 })
        );
        assert_eq!(
            cursor_to_frame(p, r, s, Rotation::Clockwise180),
            Some(Pixel { x: 37, y: 80 })
        );
        assert_eq!(
            cursor_to_frame(p, r, s, Rotation::Clockwise270),
            Some(Pixel { x: 10, y: 75 })
        );
    }
    #[test]
    fn explicit_dpi_mapping() {
        let map = DpiMapping {
            logical_origin: Point { x: -1280, y: -720 },
            physical_origin: Point { x: -1920, y: -1080 },
            dpi_x: 144,
            dpi_y: 144,
        };
        assert_eq!(
            map.to_physical(Point { x: -1180, y: -620 }),
            Ok(Point { x: -1770, y: -930 })
        );
    }
    #[test]
    fn reject_empty_overflow_dpi() {
        assert_eq!(DesktopRect::new(0, 0, 0, 10), Err(GeometryError::Empty));
        assert_eq!(
            DesktopRect::new(i32::MAX, 0, 4, 4),
            Err(GeometryError::Overflow)
        );
        let map = DpiMapping {
            logical_origin: Point { x: 0, y: 0 },
            physical_origin: Point { x: 0, y: 0 },
            dpi_x: 0,
            dpi_y: 96,
        };
        assert_eq!(
            map.to_physical(Point { x: 0, y: 0 }),
            Err(GeometryError::InvalidDpi)
        );
    }
}
