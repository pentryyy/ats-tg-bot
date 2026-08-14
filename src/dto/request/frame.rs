use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FrameData {
    frame: Vec<u8>,
}
