use std::process::{Child, Command};

use shlex::Shlex;

use crate::runfile::Step;

pub trait Run {
    fn run(&self) -> std::io::Result<Option<Child>>;
}

impl Run for Step {
    fn run(&self) -> std::io::Result<Option<Child>> {
        let cmd_args = CmdArgs::from(self.command.as_str());

        let mut proc = Command::new(cmd_args.cmd).args(cmd_args.args).spawn()?;

        std::thread::sleep(self.startup_delay);

        if self.persistent {
            Ok(Some(proc))
        } else {
            proc.wait()?;
            Ok(None)
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
struct CmdArgs {
    cmd: String,
    args: Vec<String>,
}

impl From<&str> for CmdArgs {
    fn from(value: &str) -> CmdArgs {
        let mut value = value.to_string();
        for (key, val) in std::env::vars() {
            value = value.replace(format!("${key}").as_str(), val.as_str());
        }
        value = value.replace(" \\\n", " ");

        let mut shlex = Shlex::new(value.as_str());

        let cmd = shlex.next().unwrap();
        let args = shlex.collect();

        CmdArgs { cmd, args }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_cmd_args_from_str() {
        let value = "cargo run --help";

        let expected = CmdArgs {
            cmd: "cargo".into(),
            args: ["run", "--help"].into_iter().map(String::from).collect(),
        };
        let actual = CmdArgs::from(value);

        assert_eq!(actual, expected);
    }

    #[test]
    pub fn test_cmd_args_from_str_multiline() {
        let value = r#"cargo run -- \
            --really-long true \
            --testing foo
            "#;

        let expected = CmdArgs {
            cmd: "cargo".into(),
            args: ["run", "--", "--really-long", "true", "--testing", "foo"]
                .into_iter()
                .map(String::from)
                .collect(),
        };
        let actual = CmdArgs::from(value);

        assert_eq!(actual, expected);
    }
}
