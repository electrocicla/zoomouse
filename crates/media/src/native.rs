//! Windows WGC frame adapter. GPU crop/scale, BGRA->NV12 and hardware H.264
//! encoder are deliberately NOT enabled without a validated D3D11/MF backend.
//! No CPU frame download fallback and no network-send path exist.
use crate::{
    ColorSpace, CropPlan, FrameDescriptor, InputPixelFormat, MediaAuthorization, MediaError,
    MediaSession, Size, SourceTag,
};
use windows::Graphics::DirectX::Direct3D11::Direct3DSurfaceDescription;
use windows::Graphics::DirectX::DirectXPixelFormat;
use zoomouse_capture::native::GpuFrame;

/// A real WinRT GPU surface description is checked while the WGC frame owns
/// the resource. This cannot prove same-device access, content protection,
/// SDR transfer characteristics or an encoder, so output remains disabled.
fn validate_surface_description(
    description: Direct3DSurfaceDescription,
    expected: Size,
) -> Result<(), MediaError> {
    let width = u32::try_from(description.Width).map_err(|_| MediaError::InvalidDimensions)?;
    let height = u32::try_from(description.Height).map_err(|_| MediaError::InvalidDimensions)?;
    let actual = Size::new(width, height)?;
    if actual != expected {
        return Err(MediaError::SourceChanged);
    }
    if description.Format != DirectXPixelFormat::B8G8R8A8UIntNormalized
        || description.MultisampleDescription.Count != 1
        || description.MultisampleDescription.Quality != 0
    {
        return Err(MediaError::FormatUnsupported);
    }
    Ok(())
}

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
        // Read the actual WGC-backed GPU surface, not just its caller-supplied
        // metadata. A failed description (including device loss) denies output.
        let surface = frame
            .surface
            .Description()
            .map_err(|_| MediaError::BackendUnavailable)?;
        validate_surface_description(surface, dimensions)?;
        // Revocation can race the native descriptor read.
        session.verify_output(&permit)?;
        // No GPU copy, conversion, encoder, software fallback or encoded bytes.
        Err(MediaError::BackendUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Graphics::DirectX::Direct3D11::Direct3DMultisampleDescription;

    fn description(
        width: i32,
        height: i32,
        format: DirectXPixelFormat,
    ) -> Direct3DSurfaceDescription {
        Direct3DSurfaceDescription {
            Width: width,
            Height: height,
            Format: format,
            MultisampleDescription: Direct3DMultisampleDescription {
                Count: 1,
                Quality: 0,
            },
        }
    }

    #[test]
    fn accepts_only_matching_single_sample_native_bgra_surface() {
        let expected = Size::new(1920, 1080).unwrap();
        let valid = description(1920, 1080, DirectXPixelFormat::B8G8R8A8UIntNormalized);
        assert_eq!(validate_surface_description(valid, expected), Ok(()));
        assert_eq!(
            validate_surface_description(
                description(1280, 720, DirectXPixelFormat::B8G8R8A8UIntNormalized),
                expected,
            ),
            Err(MediaError::SourceChanged)
        );
    }

    #[test]
    fn rejects_bad_native_dimensions_and_non_bgra_or_hdr_surfaces() {
        let expected = Size::new(1920, 1080).unwrap();
        for (width, height) in [(0, 1080), (-1, 1080), (10_000, 1080)] {
            assert_eq!(
                validate_surface_description(
                    description(width, height, DirectXPixelFormat::B8G8R8A8UIntNormalized),
                    expected,
                ),
                Err(MediaError::InvalidDimensions)
            );
        }
        for format in [
            DirectXPixelFormat::NV12,
            DirectXPixelFormat::R10G10B10A2UIntNormalized,
            DirectXPixelFormat::B8G8R8A8UIntNormalizedSrgb,
            DirectXPixelFormat::Unknown,
        ] {
            assert_eq!(
                validate_surface_description(description(1920, 1080, format), expected),
                Err(MediaError::FormatUnsupported)
            );
        }
    }

    #[test]
    fn refuses_multisampled_surfaces_even_when_size_and_format_match() {
        let expected = Size::new(1280, 720).unwrap();
        let mut desc = description(1280, 720, DirectXPixelFormat::B8G8R8A8UIntNormalized);
        desc.MultisampleDescription.Count = 2;
        assert_eq!(
            validate_surface_description(desc, expected),
            Err(MediaError::FormatUnsupported)
        );
        desc.MultisampleDescription.Count = 1;
        desc.MultisampleDescription.Quality = 1;
        assert_eq!(
            validate_surface_description(desc, expected),
            Err(MediaError::FormatUnsupported)
        );
    }
}
