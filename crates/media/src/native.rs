//! Windows WGC frame adapter. GPU crop/scale, BGRA->NV12 and hardware H.264
//! encoder are deliberately NOT enabled without a validated D3D11/MF backend.
//! No CPU frame download fallback and no network-send path exist.
use crate::{
    ColorSpace, CropPlan, FrameDescriptor, InputPixelFormat, MediaAuthorization, MediaError,
    MediaSession, Size, SourceTag,
};
use zoomouse_capture::native::GpuFrame;

pub struct UnavailableD3d11MediaFoundation;
impl UnavailableD3d11MediaFoundation {
    /// Validate a borrowed WGC frame and current local authority, then fail
    /// closed without consuming/copying any GPU texture or emitting H.264.
    /// Future backend must synchronously retain/copy the GPU surface while the
    /// WGC GpuFrame remains live and verify consent after encoding and sending.
    pub fn submit<A: MediaAuthorization>(
        &mut self,
        session: &mut MediaSession<A>,
        frame: &GpuFrame,
        plan: CropPlan,
        now_qpc_100ns: i64,
        trusted_color_space: ColorSpace,
    ) -> Result<(), MediaError> {
        let metadata = &frame.metadata;
        let dimensions = Size::new(metadata.size.width, metadata.size.height)?;
        let desc = FrameDescriptor {
            source: SourceTag::from_capture(&metadata.source),
            grant_generation: metadata.grant_generation,
            sequence: metadata.sequence,
            qpc_100ns: metadata.qpc_100ns,
            dimensions,
            pixel_format: InputPixelFormat::Bgra8Unorm,
            color_space: trusted_color_space,
        };
        let permit = session.admit(&desc, now_qpc_100ns, plan)?;
        session.verify_output(&permit)?;
        // No unverified media conversion, software fallback or encoded bytes.
        Err(MediaError::BackendUnavailable)
    }
}
