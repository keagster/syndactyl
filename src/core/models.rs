use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEventMessage {
    pub observer: String,
    pub event_type: String,
    pub path: String,
    pub details: Option<String>,
}
