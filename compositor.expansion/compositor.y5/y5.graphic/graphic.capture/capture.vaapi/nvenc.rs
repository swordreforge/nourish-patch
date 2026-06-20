//! NVIDIA NVENC H.264 encoder (libav `h264_nvenc`).
//!
//! NVIDIA GPUs have no VAAPI *encode* support, so the zero-copy dmabuf→VAAPI
//! path doesn't apply. Here we feed `h264_nvenc` system-memory **BGRA** frames
//! (read back from the capture entry); NVENC uploads + converts to NV12 on the
//! GPU and hardware-encodes. The encode itself is hardware (fast); the cost is
//! the GPU→CPU readback the caller performs. (A future zero-copy path would
//! import the dmabuf into CUDA.)

use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::ptr;

use crate::common::{Muxer, averr};
use crate::ffi;

pub struct NvencEncoder {
    codec_ctx: *mut ffi::AVCodecContext,
    frame: *mut ffi::AVFrame,
    mux: Muxer,
    pts: i64,
    width: u32,
    height: u32,
    temp: PathBuf,
    started: bool,
}

impl NvencEncoder {
    pub fn start(width: u32, height: u32, fps: u32) -> Option<Self> {
        let w = (width & !1) as c_int;
        let h = (height & !1) as c_int;
        if w == 0 || h == 0 {
            return None;
        }
        let fps = fps.max(1);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp = std::env::temp_dir().join(format!("y5-capture-{nanos}.mp4"));

        match unsafe { Self::init(w, h, fps, &temp) } {
            Ok((codec_ctx, frame, mux)) => Some(NvencEncoder {
                codec_ctx,
                frame,
                mux,
                pts: 0,
                width: w as u32,
                height: h as u32,
                temp,
                started: true,
            }),
            Err(e) => {
                warn!("nvenc encoder init failed ({e}); video unavailable");
                None
            }
        }
    }

    unsafe fn init(
        w: c_int,
        h: c_int,
        fps: u32,
        temp: &std::path::Path,
    ) -> Result<(*mut ffi::AVCodecContext, *mut ffi::AVFrame, Muxer), String> {
        let codec = ffi::avcodec_find_encoder_by_name(c"h264_nvenc".as_ptr());
        if codec.is_null() {
            return Err("h264_nvenc encoder not found".into());
        }
        let codec_ctx = ffi::avcodec_alloc_context3(codec);
        if codec_ctx.is_null() {
            return Err("avcodec_alloc_context3 failed".into());
        }
        {
            let c = &mut *codec_ctx;
            c.width = w;
            c.height = h;
            c.time_base = ffi::AVRational {
                num: 1,
                den: fps as c_int,
            };
            c.framerate = ffi::AVRational {
                num: fps as c_int,
                den: 1,
            };
            // BGRA from the readback → NVENC ingests BGR0 and converts to NV12.
            c.pix_fmt = ffi::AV_PIX_FMT_BGR0;
            // Reasonable defaults: p4 preset, low-latency-ish.
            ffi::av_opt_set(
                c.priv_data,
                c"preset".as_ptr(),
                c"p4".as_ptr(),
                0,
            );
        }
        let r = ffi::avcodec_open2(codec_ctx, codec, ptr::null_mut());
        if r < 0 {
            return Err(format!("avcodec_open2(h264_nvenc): {}", averr(r)));
        }

        // Reusable BGR0 frame with its own aligned buffer.
        let frame = ffi::av_frame_alloc();
        if frame.is_null() {
            return Err("av_frame_alloc failed".into());
        }
        (*frame).format = ffi::AV_PIX_FMT_BGR0;
        (*frame).width = w;
        (*frame).height = h;
        let r = ffi::av_frame_get_buffer(frame, 0);
        if r < 0 {
            return Err(format!("av_frame_get_buffer: {}", averr(r)));
        }

        let mux = Muxer::new(temp, codec_ctx)?;
        Ok((codec_ctx, frame, mux))
    }

    pub fn dims(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Encode one tightly-packed BGRA frame (`w*h*4` bytes).
    ///
    /// The capture entry can be 1px wider/taller than the encoder surface (H.264
    /// needs even dimensions, so `start` rounds down with `& !1`). We copy the
    /// even-cropped overlap rather than dropping the frame — otherwise odd-sized
    /// world/window captures would encode zero frames ("no video track").
    pub fn push(&mut self, bgra: &[u8], w: u32, h: u32) {
        if !self.started || w < self.width || h < self.height {
            return;
        }
        unsafe {
            if ffi::av_frame_make_writable(self.frame) < 0 {
                return;
            }
            let dst = (*self.frame).data[0];
            let dst_stride = (*self.frame).linesize[0] as usize;
            let src_stride = (w as usize) * 4;
            let copy_w = (self.width as usize) * 4;
            for y in 0..(self.height as usize) {
                let src = bgra.as_ptr().add(y * src_stride);
                let dstp = dst.add(y * dst_stride);
                ptr::copy_nonoverlapping(src, dstp, copy_w);
            }
            (*self.frame).pts = self.pts;
            self.pts += 1;
            self.mux.pump(self.codec_ctx, self.frame);
        }
    }

    pub fn finish(mut self) -> Option<PathBuf> {
        if !self.started {
            return None;
        }
        unsafe {
            self.mux.finish(self.codec_ctx);
        }
        self.started = false;
        Some(self.temp.clone())
    }

    pub fn discard(self) {
        let temp = self.temp.clone();
        drop(self);
        let _ = std::fs::remove_file(temp);
    }
}

impl Drop for NvencEncoder {
    fn drop(&mut self) {
        unsafe {
            if !self.frame.is_null() {
                ffi::av_frame_free(&mut self.frame);
            }
            if !self.codec_ctx.is_null() {
                ffi::avcodec_free_context(&mut self.codec_ctx);
            }
        }
        let _ = ptr::null::<c_void>();
    }
}
