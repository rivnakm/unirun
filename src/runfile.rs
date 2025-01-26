use std::collections::HashMap;

use serde::Deserialize;

pub type JobId = String;

#[derive(Clone, Debug, Deserialize)]
pub struct Runfile {
    pub default: JobId,
    pub jobs: HashMap<JobId, Job>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Job {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub needs: Vec<JobId>,

    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Step {
    #[serde(rename(deserialize = "run"))]
    pub command: String,

    #[serde(default)]
    pub persistent: bool,
}
