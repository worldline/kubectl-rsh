use clap::{arg, crate_version, ArgMatches, Command};

pub const DEFAULT_SHELL: &str = "/bin/sh";

pub struct Args {
    pub pod: String,
    pub container: Option<String>,
    pub shell: String,
    pub namespace: Option<String>,
    pub command: Option<Vec<String>>,
}

impl Args {
    pub fn parse() -> Args {
        Self::build(Self::command().get_matches())
    }

    #[cfg(test)]
    fn parse_from(args: &str) -> Args {
        Self::build(Self::command().get_matches_from(args.split(' ')))
    }

    fn build(matches: ArgMatches) -> Args {
        let pod_name = matches.get_one::<String>("pod_name").unwrap();

        Args {
            pod: pod_name.to_owned(),
            container: matches.get_one::<String>("container").map(|i| i.to_owned()),
            shell: matches
                .get_one::<String>("shell")
                .map_or(DEFAULT_SHELL.to_owned(), |i| i.to_owned()),
            namespace: matches.get_one::<String>("namespace").map(|i| i.to_owned()),
            command: matches
                .get_many::<String>("COMMAND")
                .map(|i| i.map(|s| s.to_owned()).collect()),
        }
    }

    fn command() -> Command {
        Command::new("kubectl-rsh")
            .version(crate_version!())
            .about("Executes a command or opens a remote shell session in a container in a pod.")
            .arg(arg!(<pod_name> "Pod name").required(true))
            .arg(
                arg!(-c --container <CONTAINER_NAME> "Container name; defaults to first container in the pod")
            )
            .arg(
                arg!(-s --shell <SHELL_PATH> "Path to the shell")
                    .default_value(DEFAULT_SHELL),
            )
            .arg(
                arg!(-n --namespace <NAMESPACE> "Namespace; defaults to namespace in current context")
            )
            .arg(arg!(<COMMAND> ... "Command to run on the container, in non interractive mode")
                .trailing_var_arg(true)
                .required(false),
            )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_for_interractive() {
        let sut = Args::parse_from("kubectl-rsh shell -c first -s /bin/bash -n my_space");

        assert_eq!("shell", sut.pod);
        assert_eq!(Some("first"), sut.container.as_ref().map(|i| i.as_str()));
        assert_eq!("/bin/bash", sut.shell);
        assert_eq!(Some("my_space"), sut.namespace.as_ref().map(|i| i.as_str()));
    }

    #[test]
    fn parse_for_command() {
        let sut = Args::parse_from("kubectl-rsh shell ls -lAh");

        assert_eq!("shell", sut.pod);
        assert_eq!(Some(vec!["ls".to_owned(), "-lAh".to_owned()]), sut.command);
    }
}
