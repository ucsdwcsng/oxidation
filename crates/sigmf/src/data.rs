#![allow(unused_imports)]

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct SmfData {
    pub format: String,
    pub shape: Vec<usize>,
    pub data: Vec<u8>,
}

