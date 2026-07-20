use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::sync::Once;

static INIT_ORT: Once = Once::new();

pub struct VadSession {
    session: Session,
}

impl VadSession {
    pub fn new() -> Result<Self, String> {
        INIT_ORT.call_once(|| {
            let _ = ort::init().with_name("vad").commit();
        });

        let session = Session::builder()
            .map_err(|e| format!("Session build error: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| format!("Optimization error: {}", e))?
            .commit_from_memory(include_bytes!("silero_vad.onnx"))
            .map_err(|e| format!("Commit error: {}", e))?;

        Ok(Self { session })
    }

    pub fn trim_silence(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<Option<Vec<f32>>, String> {
        const CHUNK_SIZE: usize = 512;
        const VAD_THRESHOLD: f32 = 0.5;
        let num_chunks = samples.len() / CHUNK_SIZE;
        if num_chunks == 0 {
            return Ok(None);
        }

        let mut state_data = vec![0.0f32; 2 * 1 * 128];
        let context_size = if sample_rate == 16000 { 64 } else { 32 };
        let mut context = vec![0.0f32; context_size];

        let mut first_speech_chunk = None;
        let mut last_speech_chunk = None;

        for i in 0..num_chunks {
            let chunk = &samples[i * CHUNK_SIZE..(i + 1) * CHUNK_SIZE];

            let mut input_with_context = Vec::with_capacity(context_size + CHUNK_SIZE);
            input_with_context.extend_from_slice(&context);
            input_with_context.extend_from_slice(chunk);

            context.copy_from_slice(&chunk[CHUNK_SIZE - context_size..]);

            let input_tensor = Tensor::from_array((
                vec![1i64, (CHUNK_SIZE + context_size) as i64],
                input_with_context.into_boxed_slice(),
            ))
            .map_err(|e| e.to_string())?;

            let sr_tensor = Tensor::from_array((
                Vec::<i64>::new(),
                vec![sample_rate as i64].into_boxed_slice(),
            ))
            .map_err(|e| e.to_string())?;

            let state_tensor = Tensor::from_array((
                vec![2i64, 1i64, 128i64],
                state_data.clone().into_boxed_slice(),
            ))
            .map_err(|e| e.to_string())?;

            let inputs = ort::inputs![
                "input" => input_tensor,
                "sr" => sr_tensor,
                "state" => state_tensor,
            ];

            let mut outputs = self.session.run(inputs).map_err(|e| e.to_string())?;

            let probability = {
                let out = outputs.remove("output").unwrap();
                let (_, data) = out.try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
                *data.first().unwrap_or(&0.0)
            };

            let new_state = {
                let state_out = outputs.remove("stateN").unwrap();
                let (_, data) = state_out
                    .try_extract_tensor::<f32>()
                    .map_err(|e| e.to_string())?;
                data.to_vec()
            };

            state_data = new_state;

            if probability > VAD_THRESHOLD {
                if first_speech_chunk.is_none() {
                    first_speech_chunk = Some(i);
                }
                last_speech_chunk = Some(i);
            }
        }

        println!(
            "VAD processing {} chunks. First speech: {:?}, Last speech: {:?}",
            num_chunks, first_speech_chunk, last_speech_chunk
        );

        let first = match first_speech_chunk {
            Some(f) => f,
            None => return Ok(None),
        };
        let last = last_speech_chunk.unwrap();

        // Add ~200ms padding
        let padding_chunks = (sample_rate / CHUNK_SIZE as u32 / 5) as usize;

        let start_chunk = first.saturating_sub(padding_chunks);
        let end_chunk = (last + padding_chunks + 1).min(num_chunks);

        let start_sample = start_chunk * CHUNK_SIZE;
        let end_sample = end_chunk * CHUNK_SIZE;

        Ok(Some(samples[start_sample..end_sample].to_vec()))
    }
}
