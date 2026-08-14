use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioData {
    pub mic1: Vec<i16>,
    pub mic2: Vec<i16>,
}
