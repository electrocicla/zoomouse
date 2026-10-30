//! Windows GPU media security and format contracts. No encoder is activated here.
//! The default authority denies all work. The host must authenticate and recheck
//! its local device/session/generation grant. Neither source tags nor network
//! fields authorize frames. No CPU pixel readback or network operations exist.
#![forbid(unsafe_code)]

#[cfg(windows)]
pub mod native;

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use zoomouse_capture::{CaptureScope, SourceId, SourceKind};

const MAX_SOURCE_PIXELS: u64 = 36_000_000;
const MAX_OUTPUT_PIXELS: u64 = 1920 * 1080;
const MAX_AGE_100NS: i64 = 5_000_000; // 500ms, not wall-clock
const MAX_FUTURE_100NS: i64 = 10_000; // 1ms QPC tolerance

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaError {
    InvalidDimensions,
    InvalidCrop,
    ChromaAlignment,
    AspectMismatch,
    InvalidProfile,
    InvalidIdentity,
    Denied,
    Revoked,
    SourceChanged,
    GenerationChanged,
    FormatUnsupported,
    ReplayOrReorder,
    StaleFrame,
    FutureFrame,
    Backpressure,
    ForeignLease,
    BackendUnavailable,
}
impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for MediaError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}
impl Size {
    pub fn new(width: u32, height: u32) -> Result<Self, MediaError> {
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS
        {
            return Err(MediaError::InvalidDimensions);
        }
        Ok(Self { width, height })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropRect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}
impl CropRect {
    pub fn new(
        left: u32,
        top: u32,
        width: u32,
        height: u32,
        source: Size,
    ) -> Result<Self, MediaError> {
        Size::new(width, height)?;
        if !left.is_multiple_of(2)
            || !top.is_multiple_of(2)
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
        {
            return Err(MediaError::ChromaAlignment);
        }
        if left.checked_add(width).is_none_or(|x| x > source.width)
            || top.checked_add(height).is_none_or(|y| y > source.height)
        {
            return Err(MediaError::InvalidCrop);
        }
        Ok(Self {
            left,
            top,
            width,
            height,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropPlan {
    pub source: Size,
    pub crop: CropRect,
    pub output: Size,
}
impl CropPlan {
    pub fn new(source: Size, crop: CropRect, output: Size) -> Result<Self, MediaError> {
        CropRect::new(crop.left, crop.top, crop.width, crop.height, source)?;
        if !output.width.is_multiple_of(2)
            || !output.height.is_multiple_of(2)
            || u64::from(output.width) * u64::from(output.height) > MAX_OUTPUT_PIXELS
        {
            return Err(MediaError::InvalidDimensions);
        }
        if u64::from(crop.width) * u64::from(output.height)
            != u64::from(crop.height) * u64::from(output.width)
        {
            return Err(MediaError::AspectMismatch);
        }
        Ok(Self {
            source,
            crop,
            output,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeProfile {
    pub output: Size,
    pub fps: u8,
    pub bitrate_bps: u32,
}
impl EncodeProfile {
    pub fn new(output: Size, fps: u8, bitrate_bps: u32) -> Result<Self, MediaError> {
        if !output.width.is_multiple_of(2)
            || !output.height.is_multiple_of(2)
            || u64::from(output.width) * u64::from(output.height) > MAX_OUTPUT_PIXELS
            || !(1..=60).contains(&fps)
            || !(250_000..=20_000_000).contains(&bitrate_bps)
        {
            return Err(MediaError::InvalidProfile);
        }
        Ok(Self {
            output,
            fps,
            bitrate_bps,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputPixelFormat {
    Bgra8Unorm,
    Nv12,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    SdrBt709Limited,
    SdrBt601,
    HdrPqBt2020,
    Unknown,
}

/// Source identity is informational, never a consent grant. Construct only from
/// the local WGC picker source when accepting a real GpuFrame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTag {
    pub kind: SourceKind,
    pub local_sequence: u64,
}
impl SourceTag {
    pub fn from_capture(source: &SourceId) -> Self {
        Self {
            kind: source.kind(),
            local_sequence: source.local_sequence(),
        }
    }
    pub fn new(kind: SourceKind, local_sequence: u64) -> Result<Self, MediaError> {
        if local_sequence == 0 {
            return Err(MediaError::InvalidIdentity);
        }
        Ok(Self {
            kind,
            local_sequence,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameDescriptor {
    pub source: SourceTag,
    pub grant_generation: u64,
    pub sequence: u64,
    pub qpc_100ns: i64,
    pub dimensions: Size,
    pub pixel_format: InputPixelFormat,
    pub color_space: ColorSpace,
}

/// Only the authenticated Windows user-session host can implement a positive
/// decision: inspect current peer, OS lock state, source and grant generation.
/// Never implement from renderer flags, packet JSON or an Android claim.
pub trait MediaAuthorization {
    fn permits(&self, scope: &CaptureScope, source: SourceTag) -> bool;
}
#[derive(Debug, Default, Clone, Copy)]
pub struct DenyAll;
impl MediaAuthorization for DenyAll {
    fn permits(&self, _: &CaptureScope, _: SourceTag) -> bool {
        false
    }
}

#[derive(Default)]
struct FrameClock {
    last: Option<(u64, i64)>,
}
impl FrameClock {
    fn check(&self, sequence: u64, qpc: i64, now: i64) -> Result<(), MediaError> {
        if qpc <= 0 || now <= 0 {
            return Err(MediaError::StaleFrame);
        }
        if i128::from(qpc) - i128::from(now) > i128::from(MAX_FUTURE_100NS) {
            return Err(MediaError::FutureFrame);
        }
        if i128::from(now) - i128::from(qpc) > i128::from(MAX_AGE_100NS) {
            return Err(MediaError::StaleFrame);
        }
        if let Some((prior_sequence, prior_qpc)) = self.last {
            if sequence <= prior_sequence || qpc <= prior_qpc {
                return Err(MediaError::ReplayOrReorder);
            }
        }
        Ok(())
    }
    fn commit(&mut self, sequence: u64, qpc: i64) {
        self.last = Some((sequence, qpc));
    }
}

struct SlotState {
    used: AtomicUsize,
    capacity: usize,
}
#[derive(Clone)]
pub struct SlotPool(Arc<SlotState>);
impl SlotPool {
    pub fn new(capacity: usize) -> Result<Self, MediaError> {
        if !(1..=3).contains(&capacity) {
            return Err(MediaError::InvalidProfile);
        }
        Ok(Self(Arc::new(SlotState {
            used: AtomicUsize::new(0),
            capacity,
        })))
    }
    fn reserve(&self) -> Result<Slot, MediaError> {
        self.0
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < self.0.capacity).then_some(used + 1)
            })
            .map_err(|_| MediaError::Backpressure)?;
        Ok(Slot(Arc::clone(&self.0)))
    }
    pub fn in_flight(&self) -> usize {
        self.0.used.load(Ordering::Acquire)
    }
}
struct Slot(Arc<SlotState>);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.used.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Admission lease represents one bounded in-flight GPU job, not a texture,
/// encoded frame, or permission to send. Dropping it releases the slot.
pub struct FrameLease {
    _slot: Slot,
    session_alive: Arc<AtomicBool>,
    source: SourceTag,
    generation: u64,
}

pub struct MediaSession<A: MediaAuthorization> {
    authority: A,
    scope: CaptureScope,
    source: SourceTag,
    dimensions: Size,
    profile: EncodeProfile,
    clock: FrameClock,
    slots: SlotPool,
    alive: Arc<AtomicBool>,
}
impl<A: MediaAuthorization> MediaSession<A> {
    pub fn new(
        authority: A,
        scope: CaptureScope,
        source: SourceTag,
        dimensions: Size,
        profile: EncodeProfile,
        in_flight_limit: usize,
    ) -> Result<Self, MediaError> {
        if source.local_sequence == 0 || scope.generation() == 0 {
            return Err(MediaError::InvalidIdentity);
        }
        // Constructor cannot grant consent. A current host check remains required per frame.
        Ok(Self {
            authority,
            scope,
            source,
            dimensions,
            profile,
            clock: FrameClock::default(),
            slots: SlotPool::new(in_flight_limit)?,
            alive: Arc::new(AtomicBool::new(true)),
        })
    }
    fn ensure_current(&self) -> Result<(), MediaError> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(MediaError::Revoked);
        }
        if !self.authority.permits(&self.scope, self.source) {
            return Err(MediaError::Denied);
        }
        Ok(())
    }
    /// Pure metadata admission: no GPU copy, encoding or transport. The real
    /// native backend must retain the source frame throughout GPU consumption.
    pub fn admit(
        &mut self,
        frame: &FrameDescriptor,
        now_qpc_100ns: i64,
        plan: CropPlan,
    ) -> Result<FrameLease, MediaError> {
        self.ensure_current()?;
        if frame.source != self.source {
            return Err(MediaError::SourceChanged);
        }
        if frame.grant_generation != self.scope.generation() {
            return Err(MediaError::GenerationChanged);
        }
        if frame.dimensions != self.dimensions || plan.source != self.dimensions {
            return Err(MediaError::SourceChanged);
        }
        CropPlan::new(plan.source, plan.crop, plan.output)?;
        if plan.output != self.profile.output {
            return Err(MediaError::InvalidProfile);
        }
        if frame.pixel_format != InputPixelFormat::Bgra8Unorm
            || frame.color_space != ColorSpace::SdrBt709Limited
        {
            return Err(MediaError::FormatUnsupported);
        }
        self.clock
            .check(frame.sequence, frame.qpc_100ns, now_qpc_100ns)?;
        let slot = self.slots.reserve()?;
        // Deny if grant/desktop state changed during reservation.
        self.ensure_current()?;
        self.clock.commit(frame.sequence, frame.qpc_100ns);
        Ok(FrameLease {
            _slot: slot,
            session_alive: Arc::clone(&self.alive),
            source: self.source,
            generation: self.scope.generation(),
        })
    }
    /// Must be called after a native GPU/codec operation and immediately before
    /// any separately authorized network send; a lease is NOT send authority.
    pub fn verify_output(&self, lease: &FrameLease) -> Result<(), MediaError> {
        if !Arc::ptr_eq(&lease.session_alive, &self.alive)
            || lease.source != self.source
            || lease.generation != self.scope.generation()
        {
            return Err(MediaError::ForeignLease);
        }
        self.ensure_current()
    }
    pub fn revoke(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
    pub fn in_flight(&self) -> usize {
        self.slots.in_flight()
    }
}
impl<A: MediaAuthorization> Drop for MediaSession<A> {
    fn drop(&mut self) {
        self.revoke();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct DynamicAuthority(Arc<AtomicBool>);
    impl MediaAuthorization for DynamicAuthority {
        fn permits(&self, _: &CaptureScope, _: SourceTag) -> bool {
            self.0.load(Ordering::Acquire)
        }
    }
    fn source() -> SourceTag {
        SourceTag::new(SourceKind::UserPicked, 7).unwrap()
    }
    fn size() -> Size {
        Size::new(1920, 1080).unwrap()
    }
    fn profile() -> EncodeProfile {
        EncodeProfile::new(Size::new(1280, 720).unwrap(), 30, 2_000_000).unwrap()
    }
    fn plan() -> CropPlan {
        let s = size();
        CropPlan::new(
            s,
            CropRect::new(0, 0, 1920, 1080, s).unwrap(),
            profile().output,
        )
        .unwrap()
    }
    fn frame() -> FrameDescriptor {
        FrameDescriptor {
            source: source(),
            grant_generation: 2,
            sequence: 1,
            qpc_100ns: 10_000_000,
            dimensions: size(),
            pixel_format: InputPixelFormat::Bgra8Unorm,
            color_space: ColorSpace::SdrBt709Limited,
        }
    }
    fn session(allow: bool, cap: usize) -> (MediaSession<DynamicAuthority>, Arc<AtomicBool>) {
        let granted = Arc::new(AtomicBool::new(allow));
        let scope = CaptureScope::new("peer-1".into(), "session-1234567890".into(), 2).unwrap();
        (
            MediaSession::new(
                DynamicAuthority(granted.clone()),
                scope,
                source(),
                size(),
                profile(),
                cap,
            )
            .unwrap(),
            granted,
        )
    }
    #[test]
    fn crop_validates_even_boundaries_and_non_distorting_aspect() {
        assert_eq!(
            CropRect::new(1, 0, 100, 100, size()),
            Err(MediaError::ChromaAlignment)
        );
        assert_eq!(
            CropRect::new(1900, 0, 100, 100, size()),
            Err(MediaError::InvalidCrop)
        );
        assert_eq!(
            CropRect::new(u32::MAX, 0, 100, 100, size()),
            Err(MediaError::ChromaAlignment)
        );
        let crop = CropRect::new(0, 0, 1280, 720, size()).unwrap();
        assert_eq!(
            CropPlan::new(size(), crop, Size::new(800, 600).unwrap()),
            Err(MediaError::AspectMismatch)
        );
    }
    #[test]
    fn sizes_and_profile_have_finite_budgets() {
        assert_eq!(Size::new(0, 1080), Err(MediaError::InvalidDimensions));
        assert_eq!(Size::new(8192, 8192), Err(MediaError::InvalidDimensions));
        assert_eq!(
            EncodeProfile::new(Size::new(1280, 720).unwrap(), 0, 2_000_000),
            Err(MediaError::InvalidProfile)
        );
        assert_eq!(
            EncodeProfile::new(Size::new(1280, 720).unwrap(), 30, 0),
            Err(MediaError::InvalidProfile)
        );
    }
    #[test]
    fn explicit_deny_before_any_frame() {
        let (mut s, _) = session(false, 2);
        assert!(matches!(
            s.admit(&frame(), 10_000_000, plan()),
            Err(MediaError::Denied)
        ));
        assert_eq!(s.in_flight(), 0);
        let scope = CaptureScope::new("peer-1".into(), "session-1234567890".into(), 2).unwrap();
        let mut d = MediaSession::new(DenyAll, scope, source(), size(), profile(), 1).unwrap();
        assert!(matches!(
            d.admit(&frame(), 10_000_000, plan()),
            Err(MediaError::Denied)
        ));
    }
    #[test]
    fn rejects_changed_generation_source_size_and_color() {
        let (mut s, _) = session(true, 2);
        let mut f = frame();
        f.grant_generation = 3;
        assert!(matches!(
            s.admit(&f, 10_000_000, plan()),
            Err(MediaError::GenerationChanged)
        ));
        f = frame();
        f.source.local_sequence = 8;
        assert!(matches!(
            s.admit(&f, 10_000_000, plan()),
            Err(MediaError::SourceChanged)
        ));
        f = frame();
        f.dimensions = Size::new(1280, 720).unwrap();
        assert!(matches!(
            s.admit(&f, 10_000_000, plan()),
            Err(MediaError::SourceChanged)
        ));
        f = frame();
        f.color_space = ColorSpace::HdrPqBt2020;
        assert!(matches!(
            s.admit(&f, 10_000_000, plan()),
            Err(MediaError::FormatUnsupported)
        ));
        f = frame();
        f.pixel_format = InputPixelFormat::Nv12;
        assert!(matches!(
            s.admit(&f, 10_000_000, plan()),
            Err(MediaError::FormatUnsupported)
        ));
    }
    #[test]
    fn timestamp_replay_future_and_stale_rejected() {
        let (mut s, _) = session(true, 2);
        let f = frame();
        assert!(matches!(
            s.admit(&f, 20_000_001, plan()),
            Err(MediaError::StaleFrame)
        ));
        assert!(matches!(
            s.admit(&f, 9_989_999, plan()),
            Err(MediaError::FutureFrame)
        ));
        let lease = s.admit(&f, 10_000_000, plan()).unwrap();
        assert!(matches!(
            s.admit(&f, 10_000_000, plan()),
            Err(MediaError::ReplayOrReorder)
        ));
        drop(lease);
        let mut next = f;
        next.sequence = 2;
        assert!(matches!(
            s.admit(&next, 10_000_000, plan()),
            Err(MediaError::ReplayOrReorder)
        ));
    }
    #[test]
    fn bounded_in_flight_and_releases_on_drop() {
        let (mut s, _) = session(true, 1);
        let one = s.admit(&frame(), 10_000_000, plan()).unwrap();
        let mut second = frame();
        second.sequence = 2;
        second.qpc_100ns += 1;
        assert!(matches!(
            s.admit(&second, 10_000_001, plan()),
            Err(MediaError::Backpressure)
        ));
        assert_eq!(s.in_flight(), 1);
        drop(one);
        let two = s.admit(&second, 10_000_001, plan()).unwrap();
        assert_eq!(s.in_flight(), 1);
        drop(two);
        assert_eq!(s.in_flight(), 0);
    }
    #[test]
    fn reject_revocation_after_admission_and_across_sessions() {
        let (mut s, grant) = session(true, 2);
        let one = s.admit(&frame(), 10_000_000, plan()).unwrap();
        grant.store(false, Ordering::Release);
        assert!(matches!(s.verify_output(&one), Err(MediaError::Denied)));
        grant.store(true, Ordering::Release);
        let (other, _) = session(true, 1);
        assert!(matches!(
            other.verify_output(&one),
            Err(MediaError::ForeignLease)
        ));
        s.revoke();
        assert!(matches!(s.verify_output(&one), Err(MediaError::Revoked)));
    }
    #[test]
    fn rejects_invalid_source_and_capacity() {
        assert!(matches!(
            SourceTag::new(SourceKind::UserPicked, 0),
            Err(MediaError::InvalidIdentity)
        ));
        assert!(matches!(SlotPool::new(0), Err(MediaError::InvalidProfile)));
        assert!(matches!(SlotPool::new(4), Err(MediaError::InvalidProfile)));
    }
}
