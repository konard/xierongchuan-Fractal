//! Collection of methods for audio.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use gst::prelude::*;
use gtk::{gio, glib, prelude::*};
use matrix_sdk::attachment::BaseAudioInfo;
use tracing::warn;

use super::load_gstreamer_media_info;
pub(crate) use super::waveform::normalize_waveform;
use crate::utils::OneshotNotifier;

/// Load information for the audio in the given file.
pub(crate) async fn load_audio_info(file: &gio::File) -> BaseAudioInfo {
    let mut info = BaseAudioInfo::default();

    if let Some(media_info) = load_gstreamer_media_info(file).await {
        info.duration = media_info.duration().map(Into::into);
    }

    info.waveform = generate_waveform(file, info.duration).await;

    info
}

/// Generate a waveform for the given audio file.
///
/// The returned waveform should contain between 30 and 110 samples with a value
/// between 0 and 1.
async fn generate_waveform(file: &gio::File, duration: Option<Duration>) -> Option<Vec<f32>> {
    // We first need to get the duration, to compute the interval required to
    // collect just enough samples. We use a separate pipeline for simplicity,
    // but we could use the same pipeline and ignore the first run while we
    // collect the duration.
    let interval = duration
        // Take 110 samples, it should more or less match the maximum number of samples we present.
        // Default to 10 samples per second.
        .map_or_else(|| Duration::from_millis(100), |duration| duration / 110);

    // Create our pipeline from a pipeline description string.
    let pipeline = match gst::parse::launch(&format!(
        "uridecodebin3 uri={} ! audioconvert ! audio/x-raw,channels=1 ! level name=level interval={} ! fakesink qos=false sync=false",
        file.uri(),
        interval.as_nanos()
    )) {
        Ok(pipeline) => pipeline
            .downcast::<gst::Pipeline>()
            .expect("GstElement should be a GstPipeline"),
        Err(error) => {
            warn!("Could not create GstPipeline for audio waveform: {error}");
            return None;
        }
    };

    let notifier = OneshotNotifier::<()>::new("generate_waveform");
    let receiver = notifier.listen();
    let samples = Arc::new(Mutex::new(vec![]));
    let bus = pipeline.bus().expect("GstPipeline should have a GstBus");

    let samples_clone = samples.clone();
    let _bus_guard = bus
        .add_watch(move |_, message| {
            match message.view() {
                gst::MessageView::Eos(_) => {
                    // We are done collecting the samples.
                    notifier.notify();
                    glib::ControlFlow::Break
                }
                gst::MessageView::Error(error) => {
                    warn!("Could not generate audio waveform: {error}");
                    notifier.notify();
                    glib::ControlFlow::Break
                }
                gst::MessageView::Element(element) => {
                    if let Some(structure) = element.structure()
                        && structure.has_name("level")
                    {
                        let rms_array = structure
                            .get::<&glib::ValueArray>("rms")
                            .expect("rms value should be a GValueArray");
                        let rms = rms_array[0]
                            .get::<f64>()
                            .expect("GValueArray value should be a double");

                        match samples_clone.lock() {
                            Ok(mut samples) => {
                                let value_db = if rms.is_nan() { 0.0 } else { rms };
                                // Convert the decibels to a relative amplitude, to get a value
                                // between 0 and 1.
                                let value = 10.0_f64.powf(value_db / 20.0);

                                samples.push(value);
                            }
                            Err(error) => {
                                warn!("Failed to lock audio waveform samples mutex: {error}");
                            }
                        }
                    }
                    glib::ControlFlow::Continue
                }
                _ => glib::ControlFlow::Continue,
            }
        })
        .expect("Adding GstBus watch should succeed");

    // Collect the samples.
    let has_error = match pipeline.set_state(gst::State::Playing) {
        Ok(_) => {
            receiver.await;
            false
        }
        Err(error) => {
            warn!("Could not start GstPipeline for audio waveform: {error}");
            true
        }
    };

    // Clean up pipeline.
    let _ = pipeline.set_state(gst::State::Null);
    bus.set_flushing(true);

    if has_error {
        return None;
    }

    let waveform = match samples.lock() {
        Ok(mut samples) => std::mem::take(&mut *samples),
        Err(error) => {
            warn!("Failed to lock audio waveform samples mutex: {error}");
            return None;
        }
    };

    Some(normalize_waveform(waveform)).filter(|waveform| !waveform.is_empty())
}
