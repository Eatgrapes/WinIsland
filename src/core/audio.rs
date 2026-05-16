use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, Stream, StreamConfig};
use realfft::RealFftPlanner;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use windows::Win32::Foundation::S_OK;
use windows::Win32::Media::Audio::{
    Endpoints::IAudioMeterInformation, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::core::Interface;

pub struct AudioProcessor {
    spectrum: Arc<Mutex<[f32; 6]>>,
    gate: Arc<AtomicU32>,
    cancel_token: CancellationToken,
}

impl AudioProcessor {
    pub fn new() -> Self {
        let spectrum = Arc::new(Mutex::new([0.0f32; 6]));
        let gate = Arc::new(AtomicU32::new(0f32.to_bits()));
        let cancel_token = CancellationToken::new();
        let processor = Self {
            spectrum,
            gate,
            cancel_token,
        };
        processor.start_capture();
        processor.start_meter_thread();
        processor
    }

    pub fn get_spectrum(&self) -> [f32; 6] {
        *self.spectrum.lock().unwrap()
    }

    fn start_meter_thread(&self) {
        let cancel = self.cancel_token.clone();
        let gate_clone = self.gate.clone();
        tokio::task::spawn_blocking(move || {
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let session_manager: Option<IAudioSessionManager2> = unsafe {
                (|| -> Option<IAudioSessionManager2> {
                    let enumerator: IMMDeviceEnumerator =
                        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
                    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
                    device.Activate(CLSCTX_ALL, None).ok()
                })()
            };
            while !cancel.is_cancelled() {
                let mut max_peak = 0.0f32;
                if let Some(ref mgr) = session_manager {
                    unsafe {
                        if let Ok(enumerator) = mgr.GetSessionEnumerator() {
                            let count = enumerator.GetCount().unwrap_or(0);
                            for i in 0..count {
                                if let Ok(session) = enumerator.GetSession(i)
                                    && let Ok(session2) = session.cast::<IAudioSessionControl2>()
                                {
                                    if session2.IsSystemSoundsSession() == S_OK {
                                        continue;
                                    }
                                    if let Ok(meter) = session.cast::<IAudioMeterInformation>()
                                        && let Ok(peak) = meter.GetPeakValue()
                                    {
                                        max_peak = max_peak.max(peak);
                                    }
                                }
                            }
                        }
                    }
                }
                let gate_val = if max_peak > 0.002 { 1.0f32 } else { 0.0f32 };
                gate_clone.store(gate_val.to_bits(), Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
    }

    fn start_capture(&self) {
        let spectrum_arc = self.spectrum.clone();
        let cancel = self.cancel_token.clone();
        let gate_clone = self.gate.clone();
        tokio::task::spawn_blocking(move || {
            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => return,
            };
            let config = match device.default_output_config() {
                Ok(c) => c,
                Err(_) => return,
            };
            let stream_config: StreamConfig = config.config();
            let stream = match config.sample_format() {
                SampleFormat::F32 => {
                    build_capture_stream::<f32>(&device, &stream_config, spectrum_arc, gate_clone)
                }
                SampleFormat::I16 => {
                    build_capture_stream::<i16>(&device, &stream_config, spectrum_arc, gate_clone)
                }
                SampleFormat::U16 => {
                    build_capture_stream::<u16>(&device, &stream_config, spectrum_arc, gate_clone)
                }
                _ => return,
            };
            if let Ok(s) = stream {
                let _ = s.play();
                while !cancel.is_cancelled() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        });
    }
}

fn build_capture_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    spectrum_arc: Arc<Mutex<[f32; 6]>>,
    gate_clone: Arc<AtomicU32>,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + Copy,
    f32: FromSample<T>,
{
    let mut planner = RealFftPlanner::<f32>::new();
    let fft_len = 1024usize;
    let fft = planner.plan_fft_forward(fft_len);
    let mut output = fft.make_output_vec();
    let mut pcm_buffer = Vec::with_capacity(fft_len);
    let mut adaptive_max = [0.1f32; 6];

    device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            for &sample in data {
                pcm_buffer.push(f32::from_sample(sample));
                if pcm_buffer.len() >= fft_len {
                    update_spectrum(
                        &mut pcm_buffer,
                        &fft,
                        &mut output,
                        &mut adaptive_max,
                        &spectrum_arc,
                        &gate_clone,
                    );
                }
            }
        },
        |err| eprintln!("Audio error: {}", err),
        None,
    )
}

fn update_spectrum(
    pcm_buffer: &mut Vec<f32>,
    fft: &Arc<dyn realfft::RealToComplex<f32>>,
    output: &mut [realfft::num_complex::Complex32],
    adaptive_max: &mut [f32; 6],
    spectrum_arc: &Arc<Mutex<[f32; 6]>>,
    gate_clone: &Arc<AtomicU32>,
) {
    let mut indata = pcm_buffer[..1024].to_vec();
    let _ = fft.process(&mut indata, output);
    let gate = f32::from_bits(gate_clone.load(Ordering::Relaxed));
    let mut raw_bins = [0.0f32; 6];
    let ranges = [(2, 8), (8, 20), (20, 50), (50, 120), (120, 280), (280, 511)];
    for (j, (start, end)) in ranges.iter().enumerate() {
        let mut sum = 0.0f32;
        sum += output[*start..*end].iter().map(|v| v.norm()).sum::<f32>();
        let avg = sum / (*end - *start) as f32;
        adaptive_max[j] = adaptive_max[j] * 0.995 + avg.max(0.01) * 0.005;
        raw_bins[j] = (avg / (adaptive_max[j] * 2.3) * gate).clamp(0.0, 1.0);
    }
    let mut final_bins = [0.0f32; 6];
    final_bins[0] = raw_bins[5] * 0.8;
    final_bins[1] = raw_bins[3] * 0.9;
    final_bins[2] = raw_bins[0] * 1.0;
    final_bins[3] = raw_bins[1] * 1.0;
    final_bins[4] = raw_bins[2] * 0.9;
    final_bins[5] = raw_bins[4] * 0.8;
    if let Ok(mut s) = spectrum_arc.try_lock() {
        *s = final_bins;
    }
    pcm_buffer.clear();
}

impl Drop for AudioProcessor {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}
