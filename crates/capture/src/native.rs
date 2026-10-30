//! Windows 11 interactive-user-session WGC implementation.
//! The host MUST call this only from a verified native UI action. The OS
//! GraphicsCapturePicker is not substitutable with remote HWND/HMONITOR input.
//! The host supplies the existing D3D11 WinRT device for zero-CPU-copy handoff.
#![allow(unsafe_code)] // Audited WinRT IInitializeWithWindow call only.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use windows::core::{IInspectable, Interface};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCapturePicker,
    GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::{IDirect3DDevice, IDirect3DSurface};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::IInitializeWithWindow;

use crate::authority::{
    ensure, CaptureAuthorization, CaptureScope, ConsentError, SourceId, SourceKind,
};
use crate::geometry::Size;

const FORMAT: DirectXPixelFormat = DirectXPixelFormat::B8G8R8A8UIntNormalized;
const BUFFERS: i32 = 2;
static NEXT_SOURCE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum CaptureError {
    Consent(ConsentError),
    Cancelled,
    Unsupported,
    NoSelection,
    InvalidSourceSize,
    Windows(windows::core::Error),
}
impl From<ConsentError> for CaptureError {
    fn from(value: ConsentError) -> Self {
        Self::Consent(value)
    }
}
impl From<windows::core::Error> for CaptureError {
    fn from(value: windows::core::Error) -> Self {
        Self::Windows(value)
    }
}
impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Consent(e) => write!(f, "{e}"),
            Self::Cancelled => write!(f, "capture cancelled"),
            Self::Unsupported => write!(f, "Windows Graphics Capture unavailable"),
            Self::NoSelection => write!(f, "no current OS-picked source"),
            Self::InvalidSourceSize => write!(f, "WGC returned invalid source dimensions"),
            Self::Windows(e) => write!(f, "Windows capture API: {e}"),
        }
    }
}
impl std::error::Error for CaptureError {}
pub type Result<T> = std::result::Result<T, CaptureError>;

/// Cloning shares cancellation state with the native host. Once cancelled it
/// cannot be rearmed: a fresh session requires a fresh controller.
#[derive(Debug, Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMetadata {
    pub source: SourceId,
    pub grant_generation: u64,
    pub sequence: u64,
    /// WGC SystemRelativeTime, in 100-ns QPC time units (not wall clock).
    pub qpc_100ns: i64,
    pub size: Size,
}

/// GPU-backed frame. Keep the underlying WGC frame alive until the D3D11
/// texture is consumed by M2-B; no CPU pixel copy or network send occurs here.
pub struct GpuFrame {
    pub metadata: FrameMetadata,
    pub surface: IDirect3DSurface,
    _wgc_frame: Direct3D11CaptureFrame,
}

struct Running {
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    item: GraphicsCaptureItem,
    device: IDirect3DDevice,
    size: SizeInt32,
    closed: Arc<AtomicBool>,
    has_frame: Arc<AtomicBool>,
    closed_token: i64,
    arrived_token: i64,
    next_sequence: u64,
}
impl Running {
    fn close(self) {
        let _ = self.pool.RemoveFrameArrived(self.arrived_token);
        let _ = self.item.RemoveClosed(self.closed_token);
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

/// The default host authority is DenyAll. A real native consent implementation
/// must verify the exact authenticated peer, grant generation and desktop state.
/// No network or Tauri command is exposed by this module.
pub struct WgcCapture<A: CaptureAuthorization> {
    authority: A,
    scope: CaptureScope,
    cancellation: Cancellation,
    selection: Option<(SourceId, GraphicsCaptureItem)>,
    running: Option<Running>,
}
impl<A: CaptureAuthorization> WgcCapture<A> {
    pub fn new(authority: A, scope: CaptureScope, cancellation: Cancellation) -> Self {
        Self {
            authority,
            scope,
            cancellation,
            selection: None,
            running: None,
        }
    }
    fn check(&self) -> Result<()> {
        if self.cancellation.cancelled() {
            return Err(CaptureError::Cancelled);
        }
        ensure(&self.authority, &self.scope)?;
        Ok(())
    }
    pub fn selected_source(&self) -> Option<&SourceId> {
        self.selection.as_ref().map(|(id, _)| id)
    }
    pub fn is_capturing(&self) -> bool {
        self.running.is_some()
    }

    /// Run from a user-initiated native UI handler with its real owner HWND.
    /// This awaits the **Windows system picker**, not a remote-supplied handle.
    /// If consent changes while the picker is open, its result is discarded.
    pub async fn select_local_source(&mut self, native_owner: HWND) -> Result<Option<SourceId>> {
        self.stop();
        self.selection = None;
        self.check()?;
        if !GraphicsCaptureSession::IsSupported()? {
            return Err(CaptureError::Unsupported);
        }
        let picker = GraphicsCapturePicker::new()?;
        let initialize: IInitializeWithWindow = picker.cast()?;
        // SAFETY: native_owner is obtained by the trusted user-session Tauri
        // window integration; COM method only associates the OS picker with
        // this owner HWND. Remote HWNDs must never reach this API.
        unsafe {
            initialize.Initialize(native_owner)?;
        }
        let operation = picker.PickSingleItemAsync()?;
        let item = operation.await?;
        self.check()?;
        let size = item.Size()?;
        if size.Width <= 0 || size.Height <= 0 {
            return Err(CaptureError::InvalidSourceSize);
        }
        let id = SourceId::local(
            SourceKind::UserPicked,
            NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed),
        );
        self.selection = Some((id.clone(), item));
        Ok(Some(id))
    }

    /// Only starts after current native consent AND OS-local source selection.
    /// Device is the M2-B D3D11 interop device; no CPU fallback is created.
    pub fn start(&mut self, device: &IDirect3DDevice) -> Result<()> {
        self.stop();
        self.check()?;
        let (_, item) = self.selection.as_ref().ok_or(CaptureError::NoSelection)?;
        let size = item.Size()?;
        if size.Width <= 0 || size.Height <= 0 {
            return Err(CaptureError::InvalidSourceSize);
        }
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(device, FORMAT, BUFFERS, size)?;
        let session = pool.CreateCaptureSession(item)?;
        let closed = Arc::new(AtomicBool::new(false));
        let has_frame = Arc::new(AtomicBool::new(false));
        let close_flag = Arc::clone(&closed);
        let closed_token = item.Closed(
            &TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
                close_flag.store(true, Ordering::Release);
                Ok(())
            }),
        )?;
        let frame_flag = Arc::clone(&has_frame);
        let arrived_token = match pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new(move |_, _| {
            frame_flag.store(true, Ordering::Release);
            Ok(())
        })) {
            Ok(token) => token,
            Err(e) => {
                let _ = item.RemoveClosed(closed_token);
                return Err(e.into());
            }
        };
        if let Err(e) = session.StartCapture() {
            let _ = pool.RemoveFrameArrived(arrived_token);
            let _ = item.RemoveClosed(closed_token);
            let _ = session.Close();
            let _ = pool.Close();
            return Err(e.into());
        }
        self.running = Some(Running {
            pool,
            session,
            item: item.clone(),
            device: device.clone(),
            size,
            closed,
            has_frame,
            closed_token,
            arrived_token,
            next_sequence: 0,
        });
        Ok(())
    }

    /// No frame leaves the trusted host without a fresh authorization check.
    /// A source resize discards one frame and recreates GPU pool dimensions.
    /// Caller must consume/drop the GPU frame before stopping or resizing.
    pub fn poll_frame(&mut self) -> Result<Option<GpuFrame>> {
        if let Err(e) = self.check() {
            self.stop();
            self.selection = None;
            return Err(e);
        }
        let Some(running) = self.running.as_mut() else {
            return Ok(None);
        };
        if running.closed.load(Ordering::Acquire) {
            self.stop();
            self.selection = None;
            return Ok(None);
        }
        if !running.has_frame.swap(false, Ordering::AcqRel) {
            return Ok(None);
        }
        let frame = match running.pool.TryGetNextFrame() {
            Ok(frame) => frame,
            Err(e) => {
                self.stop();
                return Err(e.into());
            }
        };
        let size = frame.ContentSize()?;
        if size.Width <= 0 || size.Height <= 0 {
            self.stop();
            return Err(CaptureError::InvalidSourceSize);
        }
        if size != running.size {
            drop(frame);
            if let Err(e) = running
                .pool
                .Recreate(&running.device, FORMAT, BUFFERS, size)
            {
                self.stop();
                return Err(e.into());
            }
            running.size = size;
            return Ok(None);
        }
        let surface = frame.Surface()?;
        let qpc_100ns = frame.SystemRelativeTime()?.Duration;
        let source = self
            .selection
            .as_ref()
            .ok_or(CaptureError::NoSelection)?
            .0
            .clone();
        let metadata = FrameMetadata {
            source,
            grant_generation: self.scope.generation(),
            sequence: running.next_sequence,
            qpc_100ns,
            size: Size::new(size.Width as u32, size.Height as u32)
                .map_err(|_| CaptureError::InvalidSourceSize)?,
        };
        running.next_sequence = running.next_sequence.saturating_add(1);
        Ok(Some(GpuFrame {
            metadata,
            surface,
            _wgc_frame: frame,
        }))
    }

    /// Revoke immediately by closing both WGC resources. Idempotent.
    pub fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            running.close();
        }
    }
    /// Full local teardown drops source reference: regrant must repick.
    pub fn revoke(&mut self) {
        self.cancellation.cancel();
        self.stop();
        self.selection = None;
    }
}
impl<A: CaptureAuthorization> Drop for WgcCapture<A> {
    fn drop(&mut self) {
        self.stop();
    }
}
