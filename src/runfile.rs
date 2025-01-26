use std::{collections::HashMap, time::Duration};

use serde::Deserialize;
use serde_with::{serde_as, DurationMilliSeconds};

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

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Step {
    #[serde(rename(deserialize = "run"))]
    pub command: String,

    #[serde(default)]
    pub persistent: bool,

    #[serde_as(as = "DurationMilliSeconds<u64>")]
    #[serde(default)]
    pub startup_delay: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_startup_delay() {
        let toml = r#"
            run = "foo"
            startup_delay = 20
        "#;

        let expected = Step {
            command: String::from("foo"),
            persistent: false,
            startup_delay: Duration::from_millis(20),
        };

        let step: Step = toml::from_str(toml).unwrap();

        assert_eq!(step, expected);
    }

    #[test]
    fn test_deserialize_startup_delay_default() {
        let toml = r#"
            run = "foo"
        "#;

        let expected = Step {
            command: String::from("foo"),
            persistent: false,
            startup_delay: Duration::from_millis(0),
        };

        let step: Step = toml::from_str(toml).unwrap();

        assert_eq!(step, expected);
    }
}
