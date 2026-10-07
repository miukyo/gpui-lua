use anyhow::{Context as _, Result};
use std::path::Path;
use crate::video::decoder::init_ffmpeg;

pub const AUDIO_SAMPLE_RATE: u32 = 48000;
pub const AUDIO_CHANNELS: u16 = 2;

pub struct FfmpegAudioDecoder {
    input_ctx: ffmpeg_next::format::context::Input,
    decoder: ffmpeg_next::codec::decoder::Audio,
    resampler: ffmpeg_next::software::resampling::Context,
    audio_stream_index: usize,
    sample_rate: u32,
    channels: u16,
    duration_secs: f64,
}

impl FfmpegAudioDecoder {
    pub fn open(path: &Path) -> Result<Self> {
        init_ffmpeg();

        let input_ctx = ffmpeg_next::format::input(&path)
            .with_context(|| format!("Failed to open audio file: {}", path.display()))?;

        let stream = input_ctx
            .streams()
            .best(ffmpeg_next::media::Type::Audio)
            .context("No audio stream found in file")?;

        let audio_stream_index = stream.index();
        let duration_secs = if input_ctx.duration() > 0 {
            input_ctx.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE)
        } else {
            0.0
        };

        let context = ffmpeg_next::codec::context::Context::from_parameters(stream.parameters())
            .context("Failed to create audio codec context")?;

        let decoder = context
            .decoder()
            .audio()
            .context("Failed to create audio decoder")?;

        let in_rate = decoder.rate();
        let in_format = decoder.format();
        let in_layout = decoder.channel_layout();

        let resampler = ffmpeg_next::software::resampling::Context::get(
            in_format,
            in_layout,
            in_rate,
            ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
            ffmpeg_next::ChannelLayout::STEREO,
            AUDIO_SAMPLE_RATE,
        )
        .context("Failed to create software audio resampler for 48kHz stereo float")?;

        Ok(Self {
            input_ctx,
            decoder,
            resampler,
            audio_stream_index,
            sample_rate: AUDIO_SAMPLE_RATE,
            channels: AUDIO_CHANNELS,
            duration_secs,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Decode next audio frame and resample to 48kHz stereo f32 samples into output buffer.
    pub fn read_next_samples(&mut self, output: &mut Vec<f32>) -> Result<bool> {
        let mut raw_frame = ffmpeg_next::util::frame::Audio::empty();

        if self.decoder.receive_frame(&mut raw_frame).is_ok() {
            return self.process_frame(&mut raw_frame, output);
        }

        for (stream, packet) in self.input_ctx.packets() {
            if stream.index() == self.audio_stream_index {
                self.decoder.send_packet(&packet)?;
                if self.decoder.receive_frame(&mut raw_frame).is_ok() {
                    return self.process_frame(&mut raw_frame, output);
                }
            }
        }

        self.decoder.send_eof()?;
        if self.decoder.receive_frame(&mut raw_frame).is_ok() {
            return self.process_frame(&mut raw_frame, output);
        }

        Ok(false)
    }

    fn process_frame(
        &mut self,
        raw_frame: &mut ffmpeg_next::util::frame::Audio,
        output: &mut Vec<f32>,
    ) -> Result<bool> {
        let mut resampled_frame = ffmpeg_next::util::frame::Audio::empty();
        self.resampler.run(raw_frame, &mut resampled_frame)?;

        let samples_count = resampled_frame.samples() * 2; // stereo
        if samples_count == 0 {
            return Ok(true);
        }

        let slice = unsafe {
            let data_ptr = resampled_frame.data(0).as_ptr() as *const f32;
            std::slice::from_raw_parts(data_ptr, samples_count)
        };

        output.clear();
        output.extend_from_slice(slice);
        Ok(true)
    }

    pub fn seek(&mut self, seconds: f64) -> Result<()> {
        let ts = (seconds * f64::from(ffmpeg_next::ffi::AV_TIME_BASE)) as i64;
        self.input_ctx
            .seek(ts, ..ts)
            .with_context(|| format!("Failed to seek audio to {seconds}s"))?;
        let _ = self.decoder.flush();
        Ok(())
    }

    pub fn rewind(&mut self) -> Result<()> {
        self.seek(0.0)
    }
}
/// Packet-based audio decoder for live WebRTC (Opus, AAC, etc).
pub struct FfmpegPacketAudioDecoder {
    decoder: ffmpeg_next::codec::decoder::Audio,
    resampler: Option<ffmpeg_next::software::resampling::Context>,
}

impl FfmpegPacketAudioDecoder {
    pub fn new_opus() -> Result<Self> {
        init_ffmpeg();
        let codec = ffmpeg_next::codec::decoder::find(ffmpeg_next::codec::Id::OPUS)
            .context("Opus decoder not found in FFmpeg")?;
        let ctx = ffmpeg_next::codec::context::Context::new_with_codec(codec);
        let decoder = ctx.decoder().audio()?;
        Ok(Self {
            decoder,
            resampler: None,
        })
    }

    pub fn decode_packet(&mut self, payload: &[u8], samples_out: &mut Vec<f32>) -> Result<()> {
        let packet = ffmpeg_next::Packet::copy(payload);
        let _ = self.decoder.send_packet(&packet);
        let mut raw_frame = ffmpeg_next::util::frame::Audio::empty();
        while self.decoder.receive_frame(&mut raw_frame).is_ok() {
            if self.resampler.is_none() {
                self.resampler = ffmpeg_next::software::resampling::Context::get(
                    raw_frame.format(),
                    raw_frame.channel_layout(),
                    raw_frame.rate(),
                    ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
                    ffmpeg_next::ChannelLayout::STEREO,
                    AUDIO_SAMPLE_RATE,
                ).ok();
            }

            if let Some(resampler) = self.resampler.as_mut() {
                let mut resampled_frame = ffmpeg_next::util::frame::Audio::empty();
                if resampler.run(&raw_frame, &mut resampled_frame).is_ok() {
                    let samples_count = resampled_frame.samples() * AUDIO_CHANNELS as usize;
                    let data = resampled_frame.data(0);
                    let f32_slice: &[f32] = unsafe {
                        std::slice::from_raw_parts(data.as_ptr() as *const f32, samples_count.min(data.len() / 4))
                    };
                    samples_out.extend_from_slice(f32_slice);
                }
            }
        }
        Ok(())
    }
}
unsafe impl Send for FfmpegPacketAudioDecoder {}
