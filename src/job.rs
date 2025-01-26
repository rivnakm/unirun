use std::{collections::HashMap, error::Error, fmt::Display};

use itertools::Itertools;
use petgraph::{
    acyclic::Acyclic,
    algo::toposort,
    data::Build,
    graph::{DiGraph, NodeIndex},
    visit::{DfsPostOrder, NodeFiltered},
};

use crate::{runfile::Runfile, step::Run};

#[derive(Clone, Debug)]
pub struct JobNotFoundError {
    job_id: String,
}

impl JobNotFoundError {
    pub fn new(job_id: &str) -> JobNotFoundError {
        JobNotFoundError {
            job_id: job_id.to_owned(),
        }
    }
}

impl Error for JobNotFoundError {}

impl Display for JobNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Job '{}' not defined", self.job_id)
    }
}

pub fn run_job(runfile: &Runfile, job_id: &str) -> Result<(), Box<dyn Error>> {
    use signal_hook::consts::{SIGINT, SIGTERM};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&term))?;
    signal_hook::flag::register(SIGTERM, Arc::clone(&term))?;

    let graph = collect_dependencies(runfile)?;
    let order = create_run_order(job_id, graph)?;

    let mut persistent_steps = Vec::new();

    for job in order.iter().map(|j| &runfile.jobs[j]) {
        for step in job.steps.iter() {
            if let Some(proc) = step.run()? {
                persistent_steps.push(proc);
            }
        }
    }

    'outer: while !persistent_steps.is_empty() && !term.load(Ordering::Relaxed) {
        for proc in persistent_steps.iter_mut() {
            if (proc.try_wait()?).is_some() {
                break 'outer;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    if term.load(Ordering::Relaxed) {
        println!("Exit signal received, terminating...")
    }

    for proc in persistent_steps.iter_mut() {
        if cfg!(windows) {
            proc.kill().expect("failed to kill process");
        } else {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;
            signal::kill(Pid::from_raw(proc.id() as i32), Signal::SIGTERM).unwrap();

            std::thread::sleep(std::time::Duration::from_millis(250));
        };
    }

    Ok(())
}

fn create_run_order(
    job_id: &str,
    graph: Acyclic<DiGraph<String, ()>>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let Ok(start_node) = graph
        .nodes_iter()
        .filter(|&n| graph.node_weight(n).unwrap() == job_id)
        .exactly_one()
    else {
        return Err(Box::new(JobNotFoundError::new(job_id)));
    };

    let mut reachable_nodes = vec![];
    let mut dfs = DfsPostOrder::new(&graph, start_node);
    while let Some(node) = dfs.next(&graph) {
        reachable_nodes.push(node);
    }

    let filtered_graph = NodeFiltered::from_fn(&graph, |node| reachable_nodes.contains(&node));

    let results = toposort(&filtered_graph, None).unwrap();

    Ok(results
        .into_iter()
        .rev()
        .map(|n| graph.node_weight(n).unwrap().to_owned())
        .collect())
}

fn collect_dependencies(runfile: &Runfile) -> Result<Acyclic<DiGraph<String, ()>>, Box<dyn Error>> {
    let mut deps: Acyclic<DiGraph<String, ()>> = Acyclic::new();

    let nodes = runfile
        .jobs
        .keys()
        .map(|id| (id.as_str(), deps.add_node(id.to_owned())))
        .collect::<HashMap<&str, NodeIndex>>();

    for (id, job) in runfile.jobs.iter() {
        let Some(&job_node) = nodes.get(id.as_str()) else {
            return Err(Box::new(JobNotFoundError::new(id)));
        };

        for dep in job.needs.iter() {
            let Some(&dep_node) = nodes.get(dep.as_str()) else {
                return Err(Box::new(JobNotFoundError::new(dep)));
            };

            if let Err(e) = deps.try_add_edge(job_node, dep_node, ()) {
                eprintln!("unable to add edge to acyclic graph: {:?}", e);
            }
        }
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::runfile::Job;

    use super::*;

    #[test]
    fn test_create_run_order_single_node() {
        let runfile = Runfile {
            default: String::from("start"),
            jobs: HashMap::from([(
                "start".into(),
                Job {
                    name: None,
                    needs: Vec::new(),
                    steps: Vec::new(),
                },
            )]),
        };

        let graph = collect_dependencies(&runfile).unwrap();

        assert_eq!(graph.node_count(), 1, "Incorrect number of nodes in graph");
        assert_eq!(graph.edge_count(), 0, "Incorrect number of edges in graph");

        let order = create_run_order(runfile.default.as_str(), graph).unwrap();

        assert_eq!(order.len(), 1, "Incorrect number of items in run order");
    }

    #[test]
    fn test_create_run_order_two_nodes() {
        let runfile = Runfile {
            default: String::from("start"),
            jobs: HashMap::from([
                (
                    "build".into(),
                    Job {
                        name: None,
                        needs: Vec::new(),
                        steps: Vec::new(),
                    },
                ),
                (
                    "start".into(),
                    Job {
                        name: None,
                        needs: vec!["build".into()],
                        steps: Vec::new(),
                    },
                ),
            ]),
        };

        let graph = collect_dependencies(&runfile).unwrap();

        assert_eq!(graph.node_count(), 2, "Incorrect number of nodes in graph");
        assert_eq!(graph.edge_count(), 1, "Incorrect number of edges in graph");

        let order = create_run_order(runfile.default.as_str(), graph).unwrap();

        assert_eq!(order.len(), 2, "Incorrect number of items in run order");
    }

    #[test]
    fn test_create_run_order_nodes_diverge() {
        let runfile = Runfile {
            default: String::from("start"),
            jobs: HashMap::from([
                (
                    "build".into(),
                    Job {
                        name: None,
                        needs: Vec::new(),
                        steps: Vec::new(),
                    },
                ),
                (
                    "start".into(),
                    Job {
                        name: None,
                        needs: vec!["build".into()],
                        steps: Vec::new(),
                    },
                ),
                (
                    "test".into(),
                    Job {
                        name: None,
                        needs: vec!["build".into()],
                        steps: Vec::new(),
                    },
                ),
            ]),
        };

        let graph = collect_dependencies(&runfile).unwrap();

        assert_eq!(graph.node_count(), 3, "Incorrect number of nodes in graph");
        assert_eq!(graph.edge_count(), 2, "Incorrect number of edges in graph");

        let order = create_run_order(runfile.default.as_str(), graph).unwrap();

        assert_eq!(order.len(), 2, "Incorrect number of items in run order");
        assert_eq!(order, vec![String::from("build"), String::from("start")])
    }

    #[test]
    fn test_create_run_order_chained_nodes() {
        let runfile = Runfile {
            default: String::from("start"),
            jobs: HashMap::from([
                (
                    "build".into(),
                    Job {
                        name: None,
                        needs: Vec::new(),
                        steps: Vec::new(),
                    },
                ),
                (
                    "test".into(),
                    Job {
                        name: None,
                        needs: vec!["build".into()],
                        steps: Vec::new(),
                    },
                ),
                (
                    "start".into(),
                    Job {
                        name: None,
                        needs: vec!["test".into()],
                        steps: Vec::new(),
                    },
                ),
            ]),
        };

        let graph = collect_dependencies(&runfile).unwrap();

        assert_eq!(graph.node_count(), 3, "Incorrect number of nodes in graph");
        assert_eq!(graph.edge_count(), 2, "Incorrect number of edges in graph");

        let order = create_run_order(runfile.default.as_str(), graph).unwrap();

        assert_eq!(order.len(), 3, "Incorrect number of items in run order");
        assert_eq!(
            order,
            vec![
                String::from("build"),
                String::from("test"),
                String::from("start")
            ]
        )
    }

    #[test]
    fn test_create_run_order_overlapping_needs() {
        let runfile = Runfile {
            default: String::from("start"),
            jobs: HashMap::from([
                (
                    "build".into(),
                    Job {
                        name: None,
                        needs: Vec::new(),
                        steps: Vec::new(),
                    },
                ),
                (
                    "test".into(),
                    Job {
                        name: None,
                        needs: vec!["build".into()],
                        steps: Vec::new(),
                    },
                ),
                (
                    "start".into(),
                    Job {
                        name: None,
                        needs: vec!["build".into(), "test".into()],
                        steps: Vec::new(),
                    },
                ),
            ]),
        };

        let graph = collect_dependencies(&runfile).unwrap();

        assert_eq!(graph.node_count(), 3, "Incorrect number of nodes in graph");
        assert_eq!(graph.edge_count(), 3, "Incorrect number of edges in graph");

        let order = create_run_order(runfile.default.as_str(), graph).unwrap();

        assert_eq!(order.len(), 3, "Incorrect number of items in run order");
        assert_eq!(
            order,
            vec![
                String::from("build"),
                String::from("test"),
                String::from("start")
            ]
        )
    }
}
