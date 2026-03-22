use crate::{ingest::model::Identifier, processor::config::ResizeTargets};

#[derive(Debug)]
pub enum JobApplication {
    Ping(usize),
    ImageProcess(ImageProcessJob),
}

#[derive(Debug)]
pub struct ImageProcessJob {
    pub photo_id: Identifier,
    pub image_id: String,
    pub target: ResizeTargets,
}


