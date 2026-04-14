use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct ContentItem {
    pub guid: String,
    pub name: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvVar {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}
