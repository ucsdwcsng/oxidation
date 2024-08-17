#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
use serde_json;

pub mod capture;
pub mod collection;
pub mod data;
pub mod global;
pub mod meta;

pub type Number = f64;
pub type Integer = u64;

#[derive(Debug, Default, Deserialize)]
pub struct SigMF {
    pub collection: Option<collection::SmfCollection>,
    pub recordings: Vec<SmfRecording>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SmfRecording {
    pub name: String,
    pub hash: String,
}

impl SigMF {
    pub fn from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        let sigmf: SigMF = serde_json::from_reader(reader)?;
        Ok(sigmf)
    }
}

impl SmfRecording {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data_path = path.to_string() + ".sigmf-data";
        let meta_path = path.to_string() + ".sigmf-meta";

        let data_file = std::fs::File::open(data_path)?;
        let meta_file = std::fs::File::open(meta_path)?;

        let data_reader = std::io::BufReader::new(data_file);
        let meta_reader = std::io::BufReader::new(meta_file);

        let meta: meta::SmfMeta = serde_json::from_reader(meta_reader)?;
        println!("{:#?}", meta);

        let data: data::SmfData = serde_json::from_reader(data_reader)?;

        Ok(SmfRecording { meta, data })
    }
}
