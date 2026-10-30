use super::*;
fn source(id: u64, left: i32, dpi: f64) -> SourceGeometry {
    SourceGeometry::new(
        id,
        9,
        PhysicalRect::new(left, -1080, 1920, 1080).unwrap(),
        Size::source(1920, 1080).unwrap(),
        dpi,
        dpi,
        Rotation::Unrotated,
    )
    .unwrap()
}
fn topology() -> Topology {
    Topology::new(vec![source(1, -1920, 1.5), source(2, 0, 1.0)]).unwrap()
}
fn s(stream: bool, zoom: bool, follow: bool, mag: f64, smooth: f64) -> Settings {
    Settings::new(stream, zoom, follow, mag, smooth).unwrap()
}
fn p(x: f64, y: f64) -> Point {
    Point::new(x, y).unwrap()
}
fn out() -> Size {
    Size::output(1280, 720).unwrap()
}
fn step(
    v: &mut Viewport,
    id: u64,
    setting: Settings,
    cursor: Option<Point>,
    dt: f64,
) -> Projection {
    v.step(&topology(), id, out(), setting, cursor, dt)
        .unwrap()
        .unwrap()
}
#[test]
fn zoom_off_full_frame_and_4x_edge_clamps() {
    let mut v = Viewport::default();
    assert_eq!(
        step(
            &mut v,
            1,
            s(true, false, true, 4.0, 1.0),
            Some(p(-1900.0, -1000.0)),
            16.0
        )
        .crop,
        Crop {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080
        }
    );
    let mut v = Viewport::default();
    assert_eq!(
        step(
            &mut v,
            1,
            s(true, true, true, 4.0, 1.0),
            Some(p(-1920.0, -1080.0)),
            16.0
        )
        .crop,
        Crop {
            left: 0,
            top: 0,
            width: 480,
            height: 270
        }
    );
    assert_eq!(
        step(
            &mut v,
            1,
            s(true, true, true, 4.0, 1.0),
            Some(p(-1.0, -1.0)),
            16.0
        )
        .crop,
        Crop {
            left: 1440,
            top: 810,
            width: 480,
            height: 270
        }
    );
}
#[test]
fn independent_toggles_and_center_freeze() {
    let mut v = Viewport::default();
    let first = step(
        &mut v,
        1,
        s(true, true, true, 2.0, 1.0),
        Some(p(-1800.0, -900.0)),
        16.0,
    )
    .crop;
    assert_eq!(
        step(
            &mut v,
            1,
            s(true, true, false, 2.0, 1.0),
            Some(p(-100.0, -20.0)),
            16.0
        )
        .crop,
        first
    );
    assert!(v
        .step(
            &topology(),
            1,
            out(),
            s(false, true, true, 2.0, 1.0),
            Some(p(-100.0, -20.0)),
            16.0
        )
        .unwrap()
        .is_none());
    assert_eq!(
        step(
            &mut v,
            1,
            s(true, true, false, 2.0, 1.0),
            Some(p(-100.0, -20.0)),
            16.0
        )
        .crop,
        first
    );
    assert_ne!(
        step(
            &mut v,
            1,
            s(true, true, true, 2.0, 1.0),
            Some(p(-100.0, -20.0)),
            16.0
        )
        .crop,
        first
    );
}
#[test]
fn no_cross_screen_leak() {
    let mut v = Viewport::default();
    let set = s(true, true, true, 3.0, 1.0);
    let first = step(&mut v, 1, set, Some(p(-1800.0, -900.0)), 16.0);
    let next = step(&mut v, 1, set, Some(p(200.0, -800.0)), 16.0);
    assert_eq!(first.crop, next.crop);
    assert_eq!(next.physical_to_output(p(200.0, -800.0)).unwrap(), None);
    let other = step(&mut v, 2, set, Some(p(200.0, -800.0)), 16.0);
    assert_eq!(other.source.id(), 2);
    assert_eq!(other.source.desktop().left, 0);
}
#[test]
fn output_inverse_stable_roundtrip_grid() {
    let mut v = Viewport::default();
    let proj = step(
        &mut v,
        1,
        s(true, true, true, 2.5, 1.0),
        Some(p(-760.0, -300.0)),
        16.0,
    );
    for x in (0..1280).step_by(47) {
        for y in (0..720).step_by(31) {
            let input = p(f64::from(x), f64::from(y));
            let phys = proj.output_to_physical(input).unwrap();
            let round = proj.physical_to_output(phys).unwrap().unwrap();
            assert!((round.x - input.x).abs() < 1e-9);
            assert!((round.y - input.y).abs() < 1e-9);
        }
    }
    assert_eq!(
        proj.output_to_physical(p(1280.0, 0.0)),
        Err(GeometryError::OutsideSource)
    );
    assert_eq!(
        proj.output_to_physical(p(-0.1, 0.0)),
        Err(GeometryError::OutsideSource)
    );
}
#[test]
fn smoothing_frame_rate_independent() {
    let setting = s(true, true, true, 3.0, 0.1);
    let mut fast = Viewport::default();
    let mut slow = Viewport::default();
    step(&mut fast, 1, setting, Some(p(-1700.0, -900.0)), 0.0);
    step(&mut slow, 1, setting, Some(p(-1700.0, -900.0)), 0.0);
    for _ in 0..60 {
        step(
            &mut fast,
            1,
            setting,
            Some(p(-100.0, -100.0)),
            1000.0 / 60.0,
        );
    }
    for _ in 0..30 {
        step(
            &mut slow,
            1,
            setting,
            Some(p(-100.0, -100.0)),
            1000.0 / 30.0,
        );
    }
    let a = fast.center.unwrap();
    let b = slow.center.unwrap();
    assert!((a.x - b.x).abs() < 1e-9);
    assert!((a.y - b.y).abs() < 1e-9);
}
#[test]
fn smoothing_zero_and_zero_time_do_not_move() {
    let mut v = Viewport::default();
    step(
        &mut v,
        1,
        s(true, true, true, 2.0, 0.3),
        Some(p(-1700.0, -900.0)),
        16.0,
    );
    let first = v.center;
    step(
        &mut v,
        1,
        s(true, true, true, 2.0, 0.3),
        Some(p(-100.0, -100.0)),
        0.0,
    );
    assert_eq!(v.center, first);
    step(
        &mut v,
        1,
        s(true, true, true, 2.0, 0.0),
        Some(p(-100.0, -100.0)),
        16.0,
    );
    assert_eq!(v.center, first);
}
#[test]
fn mixed_dpi_negative_desktop_coordinates() {
    let t = topology();
    let left = t.source(1).unwrap();
    let right = t.source(2).unwrap();
    let physical = left
        .logical_to_physical(p(100.0, 100.0), p(0.0, 0.0), p(-1920.0, -1080.0))
        .unwrap()
        .unwrap();
    assert_eq!(physical, p(-1770.0, -930.0));
    assert_eq!(left.frame_point(physical).unwrap(), Some(p(150.0, 150.0)));
    assert_eq!(right.frame_point(physical).unwrap(), None);
    assert_eq!(
        right
            .logical_to_physical(p(100.0, 100.0), p(0.0, 0.0), p(0.0, -1080.0))
            .unwrap(),
        Some(p(100.0, -980.0))
    );
}
#[test]
fn nonfinite_rejected_without_mutation() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(Point::new(invalid, 0.0), Err(GeometryError::NonFinite));
        assert_eq!(
            Settings::new(true, true, true, invalid, 0.2),
            Err(GeometryError::NonFinite)
        );
        assert_eq!(
            Settings::new(true, true, true, 2.0, invalid),
            Err(GeometryError::NonFinite)
        );
        let mut v = Viewport::default();
        assert_eq!(
            v.step(
                &topology(),
                1,
                out(),
                s(true, true, true, 2.0, 0.2),
                None,
                invalid
            ),
            Err(GeometryError::NonFinite)
        );
        assert!(v.center.is_none());
        assert_eq!(
            SourceGeometry::new(
                1,
                1,
                PhysicalRect::new(0, 0, 1920, 1080).unwrap(),
                Size::source(1920, 1080).unwrap(),
                invalid,
                1.0,
                Rotation::Unrotated
            ),
            Err(GeometryError::NonFinite)
        );
        assert_eq!(
            topology().source(1).unwrap().logical_to_physical(
                p(0.0, 0.0),
                Point { x: invalid, y: 0.0 },
                p(0.0, 0.0)
            ),
            Err(GeometryError::NonFinite)
        );
    }
    let mut v = Viewport::default();
    step(
        &mut v,
        1,
        s(true, true, true, 2.0, 0.2),
        Some(p(-900.0, -500.0)),
        16.0,
    );
    let prior = v.center;
    assert_eq!(
        v.step(
            &topology(),
            1,
            out(),
            s(true, true, true, 2.0, 0.2),
            Some(Point {
                x: f64::NAN,
                y: 1.0
            }),
            16.0
        ),
        Err(GeometryError::NonFinite)
    );
    assert_eq!(v.center, prior);
}
#[test]
fn invalid_input_bounds_and_topology_fail_closed() {
    assert_eq!(
        Size::output(1279, 720),
        Err(GeometryError::InvalidDimensions)
    );
    assert_eq!(
        Size::output(1920, 1200),
        Err(GeometryError::InvalidDimensions)
    );
    assert_eq!(
        PhysicalRect::new(i32::MAX, 0, 4, 1),
        Err(GeometryError::Overflow)
    );
    assert_eq!(
        PhysicalRect::new(0, 0, 0, 1),
        Err(GeometryError::InvalidDimensions)
    );
    for z in [0.0, 0.9, 4.1] {
        assert_eq!(
            Settings::new(true, true, true, z, 0.5),
            Err(GeometryError::InvalidSettings)
        );
    }
    for smooth in [-0.1, 1.1] {
        assert_eq!(
            Settings::new(true, true, true, 2.0, smooth),
            Err(GeometryError::InvalidSettings)
        );
    }
    assert!(matches!(
        Topology::new(vec![]),
        Err(GeometryError::InvalidTopology)
    ));
    let one = source(1, -1920, 1.0);
    assert!(matches!(
        Topology::new(vec![one, one]),
        Err(GeometryError::DuplicateSource)
    ));
    assert!(matches!(
        Topology::new(vec![one, source(2, -1920, 1.0)]),
        Err(GeometryError::OverlappingSources)
    ));
    let mut revision = source(2, 0, 1.0);
    revision.revision = 10;
    assert!(matches!(
        Topology::new(vec![one, revision]),
        Err(GeometryError::InvalidTopology)
    ));
    let mut v = Viewport::default();
    assert_eq!(
        v.step(
            &topology(),
            99,
            out(),
            s(true, true, true, 2.0, 0.5),
            None,
            16.0
        ),
        Err(GeometryError::UnknownSource)
    );
    assert_eq!(
        v.step(
            &topology(),
            1,
            out(),
            s(true, true, true, 2.0, 0.5),
            None,
            -1.0
        ),
        Err(GeometryError::InvalidTime)
    );
}
#[test]
fn rotation_unverified_and_aspect_mismatch_rejected() {
    let rect = PhysicalRect::new(0, 0, 1920, 1080).unwrap();
    for rotation in [
        Rotation::Clockwise90,
        Rotation::Clockwise180,
        Rotation::Clockwise270,
        Rotation::Unknown,
    ] {
        assert_eq!(
            SourceGeometry::new(
                1,
                1,
                rect,
                Size::source(1920, 1080).unwrap(),
                1.0,
                1.0,
                rotation
            ),
            Err(GeometryError::UnsupportedRotation)
        );
    }
    assert_eq!(
        SourceGeometry::new(
            1,
            1,
            rect,
            Size::source(1280, 1024).unwrap(),
            1.0,
            1.0,
            Rotation::Unrotated
        ),
        Err(GeometryError::UnsupportedTransform)
    );
    assert_eq!(
        SourceGeometry::new(
            1,
            1,
            rect,
            Size::source(1920, 1080).unwrap(),
            0.4,
            1.0,
            Rotation::Unrotated
        ),
        Err(GeometryError::InvalidDpi)
    );
    let tall = SourceGeometry::new(
        1,
        1,
        PhysicalRect::new(0, 0, 1920, 1200).unwrap(),
        Size::source(1920, 1200).unwrap(),
        1.0,
        1.0,
        Rotation::Unrotated,
    )
    .unwrap();
    let t = Topology::new(vec![tall]).unwrap();
    assert_eq!(
        Viewport::default().step(&t, 1, out(), s(true, false, false, 1.0, 0.2), None, 16.0),
        Err(GeometryError::UnsupportedFullFrame)
    );
}
#[test]
fn source_revision_reset_and_half_open_edges() {
    let mut v = Viewport::default();
    step(
        &mut v,
        1,
        s(true, true, true, 4.0, 1.0),
        Some(p(-1700.0, -900.0)),
        16.0,
    );
    let before = v.center;
    let mut changed = source(1, -1920, 1.5);
    changed.revision = 10;
    let t = Topology::new(vec![changed]).unwrap();
    let next = v
        .step(&t, 1, out(), s(true, true, false, 4.0, 0.2), None, 16.0)
        .unwrap()
        .unwrap();
    assert_eq!(next.crop.left, 720);
    assert_eq!(next.crop.top, 406);
    assert_ne!(v.center, before);
    assert_eq!(
        topology()
            .source(1)
            .unwrap()
            .frame_point(p(0.0, -500.0))
            .unwrap(),
        None
    );
    assert_eq!(
        topology()
            .source(1)
            .unwrap()
            .frame_point(p(-1920.01, -500.0))
            .unwrap(),
        None
    );
}
#[test]
fn unrepresentable_zoom_rejected() {
    let tiny = SourceGeometry::new(
        1,
        1,
        PhysicalRect::new(0, 0, 32, 18).unwrap(),
        Size::source(32, 18).unwrap(),
        1.0,
        1.0,
        Rotation::Unrotated,
    )
    .unwrap();
    let t = Topology::new(vec![tiny]).unwrap();
    assert_eq!(
        Viewport::default().step(&t, 1, out(), s(true, true, true, 4.0, 1.0), None, 16.0),
        Err(GeometryError::UnrepresentableCrop)
    );
}
#[test]
fn zoom_property_grid_even_in_frame_and_exact_aspect() {
    let mut v = Viewport::default();
    for z in 10..=40 {
        for x in [-1920.0, -1800.0, -1000.0, -1.0, 0.0, 200.0] {
            let q = step(
                &mut v,
                1,
                s(true, true, true, f64::from(z) / 10.0, 1.0),
                Some(p(x, -2.0)),
                16.0,
            );
            let c = q.crop;
            assert_eq!(c.left % 2, 0);
            assert_eq!(c.top % 2, 0);
            assert_eq!(c.width % 2, 0);
            assert_eq!(c.height % 2, 0);
            assert!(c.left + c.width <= q.source.frame().width);
            assert!(c.top + c.height <= q.source.frame().height);
            assert_eq!(
                u64::from(c.width) * u64::from(q.output.height),
                u64::from(c.height) * u64::from(q.output.width)
            );
        }
    }
}
