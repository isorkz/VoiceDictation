use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::thread::JoinHandle;

pub struct RecordingHandle {
    stop: Sender<()>,
    join: Option<JoinHandle<Result<PathBuf, String>>>,
}

impl RecordingHandle {
    pub fn stop(mut self) -> Result<PathBuf, String> {
        let _ = self.stop.send(());
        let join = self
            .join
            .take()
            .ok_or_else(|| "recording join handle missing".to_string())?;
        join.join()
            .map_err(|_| "recording thread panicked".to_string())?
    }
}

pub fn start_recording(output_wav_path: PathBuf) -> Result<RecordingHandle, String> {
    let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);

    let join = std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())?;

        let default_config = device
            .default_input_config()
            .map_err(|e| format!("failed to get default input config: {e}"))?;

        let in_sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels() as usize;

        let (tx, rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let writer = std::thread::spawn(move || writer_thread(rx, output_wav_path, in_sample_rate));

        let stream = match default_config.sample_format() {
            cpal::SampleFormat::I16 => build_stream_i16(&device, &default_config.into(), channels, tx)?,
            cpal::SampleFormat::U16 => build_stream_u16(&device, &default_config.into(), channels, tx)?,
            cpal::SampleFormat::F32 => build_stream_f32(&device, &default_config.into(), channels, tx)?,
            other => return Err(format!("unsupported sample format: {other:?}")),
        };

        stream
            .play()
            .map_err(|e| format!("failed to start input stream: {e}"))?;

        let _ = stop_rx.recv();

        drop(stream);

        writer
            .join()
            .map_err(|_| "writer thread panicked".to_string())?
    });

    Ok(RecordingHandle { stop: stop_tx, join: Some(join) })
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    tx: Sender<Vec<f32>>,
) -> Result<cpal::Stream, String> {
    let err_fn = |err| eprintln!("audio input stream error: {err}");

    device
        .build_input_stream(
            config,
            move |data: &[i16], _| {
                if channels == 0 {
                    return;
                }
                let mut mono = Vec::with_capacity(data.len() / channels);
                for frame in data.chunks_exact(channels) {
                    mono.push(frame[0] as f32 / i16::MAX as f32);
                }
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("failed to build input stream: {e}"))
}

fn build_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    tx: Sender<Vec<f32>>,
) -> Result<cpal::Stream, String> {
    let err_fn = |err| eprintln!("audio input stream error: {err}");

    device
        .build_input_stream(
            config,
            move |data: &[u16], _| {
                if channels == 0 {
                    return;
                }
                let mut mono = Vec::with_capacity(data.len() / channels);
                for frame in data.chunks_exact(channels) {
                    let centered = frame[0] as f32 - 32_768.0;
                    mono.push(centered / 32_768.0);
                }
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("failed to build input stream: {e}"))
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    tx: Sender<Vec<f32>>,
) -> Result<cpal::Stream, String> {
    let err_fn = |err| eprintln!("audio input stream error: {err}");

    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                if channels == 0 {
                    return;
                }
                let mut mono = Vec::with_capacity(data.len() / channels);
                for frame in data.chunks_exact(channels) {
                    mono.push(frame[0]);
                }
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("failed to build input stream: {e}"))
}

fn writer_thread(rx: Receiver<Vec<f32>>, output_wav_path: PathBuf, in_sample_rate: u32) -> Result<PathBuf, String> {
    let out_sample_rate = 16_000u32;
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: out_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&output_wav_path, spec)
        .map_err(|e| format!("failed to create wav {}: {e}", output_wav_path.display()))?;

    let mut resampler = LinearResampler::new(in_sample_rate, out_sample_rate);
    for chunk in rx.iter() {
        resampler.push(&chunk);
        while let Some(sample) = resampler.next() {
            let i16_sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(i16_sample)
                .map_err(|e| format!("failed to write wav sample: {e}"))?;
        }
    }

    writer
        .finalize()
        .map_err(|e| format!("failed to finalize wav: {e}"))?;

    Ok(output_wav_path)
}

struct LinearResampler {
    ratio: f64,
    pos: f64,
    buf: VecDeque<f32>,
}

impl LinearResampler {
    fn new(in_rate: u32, out_rate: u32) -> Self {
        Self {
            ratio: in_rate as f64 / out_rate as f64,
            pos: 0.0,
            buf: VecDeque::new(),
        }
    }

    fn push(&mut self, samples: &[f32]) {
        self.buf.extend(samples.iter().copied());
    }

    fn next(&mut self) -> Option<f32> {
        let i0 = self.pos.floor() as usize;
        let i1 = i0 + 1;
        if i1 >= self.buf.len() {
            return None;
        }

        let s0 = self.buf[i0];
        let s1 = self.buf[i1];
        let frac = (self.pos - i0 as f64) as f32;
        let out = s0 + (s1 - s0) * frac;

        self.pos += self.ratio;

        let drop_count = self.pos.floor() as usize;
        if drop_count > 0 {
            for _ in 0..drop_count {
                let _ = self.buf.pop_front();
            }
            self.pos -= drop_count as f64;
        }

        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::LinearResampler;

    #[test]
    fn resamples_by_integer_ratio_without_interpolation_error() {
        // 48k -> 16k is an exact 3:1 ratio.
        let mut r = LinearResampler::new(48_000, 16_000);
        let input: Vec<f32> = (0..480).map(|i| i as f32).collect();
        r.push(&input);

        let mut out = Vec::new();
        while let Some(v) = r.next() {
            out.push(v);
            if out.len() > 1_000 {
                panic!("unexpectedly large output");
            }
        }

        assert!(out.len() >= 100);
        for (i, v) in out.iter().take(100).enumerate() {
            assert_eq!(*v, (i * 3) as f32);
        }
    }
}
