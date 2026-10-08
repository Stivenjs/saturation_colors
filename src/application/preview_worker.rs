use std::{
    sync::Arc,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
};

use image::RgbaImage;

use crate::{application::apply_adjustments, domain::ColorAdjustments};

pub struct PreviewWorker {
    requests: Sender<PreviewRequest>,
    results: Receiver<PreviewResult>,
}

pub struct PreviewRequest {
    pub generation: u64,
    pub image: Arc<RgbaImage>,
    pub adjustments: ColorAdjustments,
}

pub struct PreviewResult {
    pub generation: u64,
    pub image: Result<RgbaImage, crate::error::AppError>,
}

impl PreviewWorker {
    pub fn new() -> Self {
        let (request_sender, request_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        thread::Builder::new()
            .name("preview-worker".to_string())
            .spawn(move || run(request_receiver, result_sender))
            .expect("preview worker must start");
        Self {
            requests: request_sender,
            results: result_receiver,
        }
    }

    pub fn request(&self, request: PreviewRequest) {
        let _ = self.requests.send(request);
    }

    pub fn try_recv_latest(&self) -> Option<PreviewResult> {
        let mut latest = None;
        loop {
            match self.results.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

fn run(receiver: Receiver<PreviewRequest>, sender: Sender<PreviewResult>) {
    while let Ok(mut request) = receiver.recv() {
        while let Ok(newer_request) = receiver.try_recv() {
            request = newer_request;
        }
        let generation = request.generation;
        let image = apply_adjustments(&request.image, request.adjustments);
        if sender.send(PreviewResult { generation, image }).is_err() {
            break;
        }
    }
}
