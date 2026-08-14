use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameData {
    pub frame: Vec<u8>,
}
