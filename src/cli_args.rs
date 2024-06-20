use clap::Parser;

pub const DEFAULT_SHELL: &str = "/bin/sh";

/// Open a remote shell session to a container in a pod.
#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[arg(help = "Pod name")]
    pub pod_name: String,
    #[arg(help = "Container name; defaults to first container in the pod")]
    pub container_name: Option<String>,
    #[arg(
        name = "shell",
        short,
        long,
        help = "Path to the shell",
        default_value = DEFAULT_SHELL,
    )]
    pub shell_path: String,
    #[arg(
        name = "namespace",
        short,
        long,
        help = "Namespace; defaults to namespace in current context"
    )]
    pub namespace: Option<String>,
}
