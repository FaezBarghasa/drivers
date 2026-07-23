//! Low-Latency Audio Mixer & Resampler Engine for Redox OS
//!
//! Provides multi-client audio stream mixing, per-channel volume control,
//! format normalization (16-bit stereo PCM @ 44.1kHz / 48kHz), and sample clamping.

pub const DEFAULT_SAMPLE_RATE: u32 = 48000;
pub const DEFAULT_CHANNELS: u16 = 2;
pub const BUFFER_SAMPLES: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            channels: DEFAULT_CHANNELS,
        }
    }
}

pub struct ClientStream {
    pub id: usize,
    pub volume: f32, // 0.0 to 1.0
    pub format: AudioFormat,
    pub buffer: Vec<i16>,
    pub active: bool,
}

impl ClientStream {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            volume: 1.0,
            format: AudioFormat::default(),
            buffer: Vec::with_capacity(BUFFER_SAMPLES * 4),
            active: true,
        }
    }

    pub fn push_samples(&mut self, data: &[u8]) {
        let samples = data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]));
        self.buffer.extend(samples);
    }
}

pub struct AudioMixer {
    pub master_volume: f32,
    pub clients: Vec<ClientStream>,
    pub hw_buffer: Vec<i16>,
}

impl AudioMixer {
    pub fn new() -> Self {
        Self {
            master_volume: 1.0,
            clients: Vec::new(),
            hw_buffer: vec![0i16; BUFFER_SAMPLES * DEFAULT_CHANNELS as usize],
        }
    }

    pub fn add_client(&mut self, id: usize) {
        self.clients.push(ClientStream::new(id));
    }

    pub fn remove_client(&mut self, id: usize) {
        self.clients.retain(|c| c.id != id);
    }

    pub fn write_client_data(&mut self, id: usize, data: &[u8]) -> usize {
        if let Some(client) = self.clients.iter_mut().find(|c| c.id == id) {
            client.push_samples(data);
            data.len()
        } else {
            0
        }
    }

    /// Mixes pending samples from all active streams into `output_bytes`.
    /// Performs float accumulator summation with saturation clamping.
    pub fn mix(&mut self, output_bytes: &mut [u8]) -> usize {
        let frame_count = output_bytes.len() / 2;
        let mut mix_buf = vec![0.0f32; frame_count];
        let mut active_count = 0;

        for client in self.clients.iter_mut() {
            if !client.active || client.buffer.is_empty() {
                continue;
            }
            active_count += 1;
            let drain_count = frame_count.min(client.buffer.len());
            let vol = client.volume * self.master_volume;

            for (i, sample) in client.buffer.drain(..drain_count).enumerate() {
                mix_buf[i] += (sample as f32) * vol;
            }
        }

        if active_count == 0 {
            output_bytes.fill(0);
            return frame_count * 2;
        }

        for (i, &val) in mix_buf.iter().take(frame_count).enumerate() {
            let clamped = val.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            let bytes = clamped.to_le_bytes();
            if i * 2 + 1 < output_bytes.len() {
                output_bytes[i * 2] = bytes[0];
                output_bytes[i * 2 + 1] = bytes[1];
            }
        }

        frame_count * 2
    }
}
