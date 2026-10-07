use anyhow::{Context as _, Result};
use std::path::Path;
use std::sync::OnceLock;

static FFMPEG_INIT: OnceLock<()> = OnceLock::new();

pub fn init_ffmpeg() {
    FFMPEG_INIT.get_or_init(|| {
        let _ = ffmpeg_next::init();
        let _ = ffmpeg_next::format::network::init();
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::level::Level::Error);
    });
}

struct HwDecodeState {
    hw_attempted: bool,
}

unsafe extern "C" fn get_format_callback(
    s: *mut ffmpeg_next::ffi::AVCodecContext,
    fmt: *const ffmpeg_next::ffi::AVPixelFormat,
) -> ffmpeg_next::ffi::AVPixelFormat {
    unsafe {
        if s.is_null() || fmt.is_null() {
            return ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_NONE;
        }

        let hw_tried = if !(*s).opaque.is_null() {
            let state = &mut *((*s).opaque as *mut HwDecodeState);
            let prev = state.hw_attempted;
            state.hw_attempted = true;
            prev
        } else {
            false
        };

        if !hw_tried && !(*s).hw_device_ctx.is_null() {
            let hw_device_ctx =
                (*(*s).hw_device_ctx).data as *mut ffmpeg_next::ffi::AVHWDeviceContext;
            let device_type = if !hw_device_ctx.is_null() {
                (*hw_device_ctx).type_
            } else {
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE
            };

            let expected_hw_fmt = match device_type {
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA => {
                    Some(ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_D3D11)
                }
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_DXVA2 => {
                    Some(ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_DXVA2_VLD)
                }
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_D3D12VA => {
                    Some(ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_D3D12)
                }
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI => {
                    Some(ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_VAAPI)
                }
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX => {
                    Some(ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX)
                }
                ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_CUDA => {
                    Some(ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_CUDA)
                }
                _ => None,
            };

            if let Some(target_fmt) = expected_hw_fmt {
                let mut cur = fmt;
                while *cur != ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_NONE {
                    if *cur == target_fmt {
                        return *cur;
                    }
                    cur = cur.add(1);
                }
            }
        }

        // Fallback: negotiate first available software pixel format
        let mut cur = fmt;
        while *cur != ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_NONE {
            let desc = ffmpeg_next::ffi::av_pix_fmt_desc_get(*cur);
            if !desc.is_null()
                && ((*desc).flags & (ffmpeg_next::ffi::AV_PIX_FMT_FLAG_HWACCEL as u64) == 0)
            {
                return *cur;
            }
            cur = cur.add(1);
        }

        if (*s).sw_pix_fmt != ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_NONE {
            (*s).sw_pix_fmt
        } else {
            *fmt
        }
    }
}

pub struct FfmpegVideoDecoder {
    input_ctx: ffmpeg_next::format::context::Input,
    decoder: ffmpeg_next::codec::decoder::Video,
    scaler: ffmpeg_next::software::scaling::Context,
    video_stream_index: usize,
    width: u32,
    height: u32,
    fps: f64,
    duration_secs: f64,
}

impl FfmpegVideoDecoder {
    pub fn open(path: &Path) -> Result<Self> {
        init_ffmpeg();

        let input_ctx = ffmpeg_next::format::input(&path)
            .with_context(|| format!("Failed to open video file: {}", path.display()))?;

        let stream = input_ctx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .context("No video stream found in file")?;

        let video_stream_index = stream.index();
        let fps = f64::from(stream.avg_frame_rate());
        let fps = if fps > 0.0 { fps } else { 30.0 };

        let duration_secs = if input_ctx.duration() > 0 {
            input_ctx.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE)
        } else {
            0.0
        };

        let mut context = ffmpeg_next::codec::context::Context::from_parameters(stream.parameters())
            .context("Failed to create codec context from stream parameters")?;

        // Enable multithreaded slice decoding on CPU if HW acceleration is not used
        context.set_threading(ffmpeg_next::codec::threading::Config {
            kind: ffmpeg_next::codec::threading::Type::Frame,
            count: 0, // auto-detect thread count
        });

        // Set up get_format callback for hardware acceleration negotiation and automatic software fallback
        let raw_ctx_ptr = unsafe { context.as_mut_ptr() };
        unsafe {
            if !raw_ctx_ptr.is_null() {
                let state_box = Box::new(HwDecodeState { hw_attempted: false });
                (*raw_ctx_ptr).opaque = Box::into_raw(state_box) as *mut std::ffi::c_void;
                (*raw_ctx_ptr).get_format = Some(get_format_callback);
            }
        }

        // Attempt to attach hardware device context if supported
        Self::try_attach_hw_context(&mut context);

        let decoder = match context.decoder().video() {
            Ok(d) => d,
            Err(e) => {
                unsafe {
                    if !raw_ctx_ptr.is_null() && !(*raw_ctx_ptr).opaque.is_null() {
                        let _ = Box::from_raw((*raw_ctx_ptr).opaque as *mut HwDecodeState);
                        (*raw_ctx_ptr).opaque = std::ptr::null_mut();
                    }
                }
                return Err(e).context("Failed to create video decoder");
            }
        };
        let width = decoder.width();
        let height = decoder.height();

        let scaler = ffmpeg_next::software::scaling::Context::get(
            decoder.format(),
            width,
            height,
            ffmpeg_next::format::Pixel::BGRA,
            width,
            height,
            ffmpeg_next::software::scaling::Flags::FAST_BILINEAR,
        )
        .context("Failed to create software video scaler for RGBA conversion")?;

        Ok(Self {
            input_ctx,
            decoder,
            scaler,
            video_stream_index,
            width,
            height,
            fps,
            duration_secs,
        })
    }

    fn try_attach_hw_context(context: &mut ffmpeg_next::codec::context::Context) {
        #[cfg(target_os = "windows")]
        let candidate_types = [
            ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA,
            ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_DXVA2,
            ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_D3D12VA,
        ];
        #[cfg(target_os = "linux")]
        let candidate_types = [
            ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
        ];
        #[cfg(target_os = "macos")]
        let candidate_types = [
            ffmpeg_next::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX,
        ];
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return;

        unsafe {
            for hw_type in candidate_types {
                let mut hw_device_ctx: *mut ffmpeg_next::ffi::AVBufferRef = std::ptr::null_mut();
                if ffmpeg_next::ffi::av_hwdevice_ctx_create(
                    &mut hw_device_ctx,
                    hw_type,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    0,
                ) == 0
                    && !hw_device_ctx.is_null()
                {
                    let raw_ctx = context.as_mut_ptr();
                    if !raw_ctx.is_null() {
                        (*raw_ctx).hw_device_ctx = ffmpeg_next::ffi::av_buffer_ref(hw_device_ctx);
                        let name_ptr = ffmpeg_next::ffi::av_hwdevice_get_type_name(hw_type);
                        let name = if !name_ptr.is_null() {
                            std::ffi::CStr::from_ptr(name_ptr).to_string_lossy()
                        } else {
                            "hardware".into()
                        };
                        log::info!("Initialized FFmpeg hardware video decoding with -hwaccel {name} (DirectX 12 zero-copy WGPU pipeline)");
                    }
                    ffmpeg_next::ffi::av_buffer_unref(&mut hw_device_ctx);
                    break;
                }
            }
        }
    }
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Read next video frame and convert to RGBA into output buffer.
    pub fn read_next_frame(&mut self, output: &mut Vec<u8>) -> Result<bool> {
        let mut raw_frame = ffmpeg_next::util::frame::Video::empty();

        // 1. Try to receive any pending frame from the decoder
        if self.decoder.receive_frame(&mut raw_frame).is_ok() {
            return self.process_frame(&mut raw_frame, output);
        }

        // 2. Read packets from input until a frame is produced or EOF
        for (stream, packet) in self.input_ctx.packets() {
            if stream.index() == self.video_stream_index {
                let _ = self.decoder.send_packet(&packet);
                if self.decoder.receive_frame(&mut raw_frame).is_ok() {
                    return self.process_frame(&mut raw_frame, output);
                }
            }
        }

        // 3. Flush remaining frames in decoder at EOF
        let _ = self.decoder.send_eof();
        if self.decoder.receive_frame(&mut raw_frame).is_ok() {
            return self.process_frame(&mut raw_frame, output);
        }

        Ok(false)
    }

    fn process_frame(
        &mut self,
        raw_frame: &mut ffmpeg_next::util::frame::Video,
        output: &mut Vec<u8>,
    ) -> Result<bool> {
        // Handle hardware frames by transferring to system memory if necessary
        let mut sw_frame = ffmpeg_next::util::frame::Video::empty();
        let frame_to_scale = unsafe {
            let raw_ptr = raw_frame.as_ptr();
            if !raw_ptr.is_null() && !(*raw_ptr).hw_frames_ctx.is_null() {
                if ffmpeg_next::ffi::av_hwframe_transfer_data(
                    sw_frame.as_mut_ptr(),
                    raw_ptr,
                    0,
                ) == 0 {
                    &sw_frame
                } else {
                    raw_frame
                }
            } else {
                raw_frame
            }
        };

        let mut rgba_frame = ffmpeg_next::util::frame::Video::empty();
        if let Err(e) = self.scaler.run(frame_to_scale, &mut rgba_frame) {
            if let ffmpeg_next::Error::InputChanged = e {
                self.scaler = ffmpeg_next::software::scaling::Context::get(
                    frame_to_scale.format(),
                    frame_to_scale.width(),
                    frame_to_scale.height(),
                    ffmpeg_next::format::Pixel::BGRA,
                    self.width,
                    self.height,
                    ffmpeg_next::software::scaling::Flags::FAST_BILINEAR,
                )?;
                self.scaler.run(frame_to_scale, &mut rgba_frame)?;
            } else {
                return Err(e.into());
            }
        }

        let data = rgba_frame.data(0);
        let stride = rgba_frame.stride(0);
        let width = self.width as usize;
        let height = self.height as usize;
        let row_bytes = width * 4;

        output.resize(row_bytes * height, 0);

        // Copy row-by-row respecting stride alignment
        for y in 0..height {
            let src_start = y * stride;
            let dst_start = y * row_bytes;
            if src_start + row_bytes <= data.len() {
                output[dst_start..dst_start + row_bytes]
                    .copy_from_slice(&data[src_start..src_start + row_bytes]);
            }
        }

        Ok(true)
    }

    /// Seek to position in seconds.
    pub fn seek(&mut self, seconds: f64) -> Result<()> {
        let ts = (seconds * f64::from(ffmpeg_next::ffi::AV_TIME_BASE)) as i64;
        let _ = self.input_ctx.seek(ts, ..);
        self.decoder.flush();
        Ok(())
    }

    /// Reset playback to start for looping.
    pub fn rewind(&mut self) -> Result<()> {
        let _ = self.input_ctx.seek(0, ..);
        self.decoder.flush();
        Ok(())
    }
    /// Read next video frame directly as a Direct3D 11 texture (zero-copy hardware decoding).
    #[cfg(target_os = "windows")]
    pub fn read_next_d3d11_texture(
        &mut self,
    ) -> Result<Option<windows_061::Win32::Graphics::Direct3D11::ID3D11Texture2D>> {
        use windows_061::core::Interface as _;

        let mut raw_frame = ffmpeg_next::util::frame::Video::empty();

        let got_frame = if self.decoder.receive_frame(&mut raw_frame).is_ok() {
            true
        } else {
            let mut found = false;
            for (stream, packet) in self.input_ctx.packets() {
                if stream.index() == self.video_stream_index {
                    let _ = self.decoder.send_packet(&packet);
                    if self.decoder.receive_frame(&mut raw_frame).is_ok() {
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                let _ = self.decoder.send_eof();
                self.decoder.receive_frame(&mut raw_frame).is_ok()
            } else {
                true
            }
        };

        if !got_frame {
            return Ok(None);
        }

        unsafe {
            let raw_ptr = raw_frame.as_ptr();
            if !raw_ptr.is_null()
                && (*raw_ptr).format == ffmpeg_next::ffi::AVPixelFormat::AV_PIX_FMT_D3D11 as i32
            {
                let tex_ptr = (*raw_ptr).data[0] as *mut std::ffi::c_void;
                if !tex_ptr.is_null() {
                    let unk: windows_061::core::IUnknown = std::mem::transmute_copy(&tex_ptr);
                    if let Ok(texture) = unk.cast::<windows_061::Win32::Graphics::Direct3D11::ID3D11Texture2D>() {
                        return Ok(Some(texture));
                    }
                }
            }
        }

        Ok(None)
    }
}

impl Drop for FfmpegVideoDecoder {
    fn drop(&mut self) {
        unsafe {
            let raw_ctx = self.decoder.as_mut_ptr();
            if !raw_ctx.is_null() && !(*raw_ctx).opaque.is_null() {
                let _ = Box::from_raw((*raw_ctx).opaque as *mut HwDecodeState);
                (*raw_ctx).opaque = std::ptr::null_mut();
            }
        }
    }
}
/// Packet-based video decoder for live WebRTC and streaming pipelines.
pub struct FfmpegPacketVideoDecoder {
    decoder: ffmpeg_next::codec::decoder::Video,
    scaler: Option<ffmpeg_next::software::scaling::Context>,
    width: u32,
    height: u32,
}

impl FfmpegPacketVideoDecoder {
    pub fn new_h264() -> Result<Self> {
        init_ffmpeg();
        let codec = ffmpeg_next::codec::decoder::find(ffmpeg_next::codec::Id::H264)
            .context("H264 decoder not found")?;
        let ctx = ffmpeg_next::codec::context::Context::new_with_codec(codec);
        let decoder = ctx.decoder().video()?;
        Ok(Self {
            decoder,
            scaler: None,
            width: 0,
            height: 0,
        })
    }

    pub fn new_vp8() -> Result<Self> {
        init_ffmpeg();
        let codec = ffmpeg_next::codec::decoder::find(ffmpeg_next::codec::Id::VP8)
            .context("VP8 decoder not found")?;
        let ctx = ffmpeg_next::codec::context::Context::new_with_codec(codec);
        let decoder = ctx.decoder().video()?;
        Ok(Self {
            decoder,
            scaler: None,
            width: 0,
            height: 0,
        })
    }

    pub fn decode_packet(&mut self, payload: &[u8], frames_out: &mut Vec<(u32, u32, Vec<u8>)>) -> Result<()> {
        let packet = ffmpeg_next::Packet::copy(payload);
        let _ = self.decoder.send_packet(&packet);
        let mut raw_frame = ffmpeg_next::util::frame::Video::empty();
        while self.decoder.receive_frame(&mut raw_frame).is_ok() {
            let w = raw_frame.width();
            let h = raw_frame.height();
            if w == 0 || h == 0 {
                continue;
            }
            if self.scaler.is_none() || self.width != w || self.height != h {
                self.scaler = Some(ffmpeg_next::software::scaling::Context::get(
                    raw_frame.format(),
                    w,
                    h,
                    ffmpeg_next::format::Pixel::BGRA,
                    w,
                    h,
                    ffmpeg_next::software::scaling::Flags::FAST_BILINEAR,
                )?);
                self.width = w;
                self.height = h;
            }

            if let Some(scaler) = self.scaler.as_mut() {
                let mut rgba_frame = ffmpeg_next::util::frame::Video::empty();
                if scaler.run(&raw_frame, &mut rgba_frame).is_ok() {
                    let data = rgba_frame.data(0);
                    let stride = rgba_frame.stride(0);
                    let row_bytes = (w * 4) as usize;
                    let mut output = vec![0u8; row_bytes * (h as usize)];
                    for y in 0..(h as usize) {
                        let src_start = y * stride;
                        let dst_start = y * row_bytes;
                        if src_start + row_bytes <= data.len() {
                            output[dst_start..dst_start + row_bytes]
                                .copy_from_slice(&data[src_start..src_start + row_bytes]);
                        }
                    }
                    frames_out.push((w, h, output));
                }
            }
        }
        Ok(())
    }
}
unsafe impl Send for FfmpegPacketVideoDecoder {}
