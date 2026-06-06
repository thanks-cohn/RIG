pub trait Serialize {
    fn to_json_pretty(&self, indent: usize) -> String;
}

pub trait Deserialize: Sized {
    fn from_json_str(input: &str) -> Result<Self, String>;
}

#[cfg(feature = "derive")]
pub use serde_derive::{Deserialize, Serialize};
