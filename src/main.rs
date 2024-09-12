use std::{
    env,
    error::Error,
    os::fd::{AsFd, AsRawFd},
    process::exit,
    time::Duration,
};

use futures_channel::mpsc::Sender;
use http::StatusCode;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{AttachParams, AttachedProcess, TerminalSize},
    client::UpgradeConnectionError,
    Api, Client,
};
use kubectl_rsh::{
    cli_args::{Args, DEFAULT_SHELL},
    terminal::{get_terminal_size, make_terminal_raw, restore_term_attr},
};
use log::debug;
use nix::sys::termios;
use tokio::{
    io,
    signal::unix::{self, SignalKind},
};
use tokio_stream::{wrappers::SignalStream, StreamExt as _};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Args::parse();

    let client = match get_client().await {
        Err(kube::Error::Api(err)) if err.code == 401 => {
            eprintln!("Unauthorized - are you logged in on the cluster?");
            exit(-1);
        }
        Err(e) => return Err(e.into()),
        Ok(client) => client,
    };

    let api = match args.namespace {
        Some(namespace) => Api::<Pod>::namespaced(client, &namespace),
        _ => Api::<Pod>::default_namespaced(client),
    };

    match args.command {
        Some(command) => {
            run_command_in_container(api, args.pod, args.container, args.shell, command).await
        }
        None => start_interractive_session(api, args.pod, args.container, args.shell).await,
    }
}

async fn run_command_in_container(
    api: Api<Pod>,
    pod: String,
    container: Option<String>,
    shell_path: String,
    command: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    debug!(
        "Executing command '{:?} in container '{}' in pod '{}' using shell '{}'",
        command,
        container.as_ref().map_or("[default]", |i| i.as_str()),
        pod,
        shell_path,
    );

    let attach_params = create_attach_params(AttachParams::default(), container)
        .stdout(true)
        .stderr(true);

    match api.exec(&pod, command, &attach_params).await {
        Err(kube::Error::UpgradeConnection(UpgradeConnectionError::ProtocolSwitch(
            StatusCode::NOT_FOUND,
        ))) => {
            eprintln!("Not found - check the pod's name and make sure it exists");
            exit(-1);
        }
        Err(kube::Error::UpgradeConnection(UpgradeConnectionError::ProtocolSwitch(
            StatusCode::BAD_REQUEST,
        ))) => {
            eprintln!(
                "Bad request - check the container's name and make sure it exists inside the pod"
            );
            exit(-1);
        }
        Err(e) => return Err(e.into()),
        Ok(attached_process) => print_output_to_local_terminal(attached_process).await,
    }
}

async fn print_output_to_local_terminal(
    mut attached_process: AttachedProcess,
) -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();
    let mut remote_stdout = attached_process
        .stdout()
        .expect("cannot obtain container's stdout");
    let mut stderr = io::stderr();
    let mut remote_stderr = attached_process
        .stderr()
        .expect("cannot obtain container's stderr");

    let result = tokio::select! {
        res = io::copy(&mut remote_stdout, &mut stdout) => res.map_or_else(|e| Err(Into::<Box<dyn Error>>::into(e)), |_| Ok(())),
        res = io::copy(&mut remote_stderr, &mut stderr) => res.map_or_else(|e| Err(Into::<Box<dyn Error>>::into(e)), |_| Ok(())),
        res = attached_process.join() => res.map_or_else(|e| Err(Into::<Box<dyn Error>>::into(e)), |_| Ok(())),
    };

    result
}

async fn start_interractive_session(
    api: Api<Pod>,
    pod: String,
    container: Option<String>,
    shell: String,
) -> Result<(), Box<dyn Error>> {
    debug!(
        "Starting remote shell '{}' on container '{}' in pod '{}' with shell",
        shell,
        container.as_ref().map_or("[default]", |i| i.as_str()),
        pod
    );

    let attach_params = create_attach_params(AttachParams::interactive_tty(), container);
    let command = create_command(&shell);

    let attached_process = match api.exec(&pod, command, &attach_params).await {
        Err(kube::Error::UpgradeConnection(UpgradeConnectionError::ProtocolSwitch(
            StatusCode::NOT_FOUND,
        ))) => {
            eprintln!("Not found - check the pod's name and make sure it exists");
            exit(-1);
        }
        Err(kube::Error::UpgradeConnection(UpgradeConnectionError::ProtocolSwitch(
            StatusCode::BAD_REQUEST,
        ))) => {
            eprintln!(
                "Bad request - check the container's name and make sure it exists inside the pod"
            );
            exit(-1);
        }
        Err(e) => return Err(e.into()),
        Ok(attached_process) => attached_process,
    };

    let res = wire_local_terminal(attached_process).await;

    match res {
        Err(e) => Err(e.into()),
        _ => exit(0), // Exit as the stdin still hangs waiting for input
    }
}

async fn wire_local_terminal(mut attached_process: AttachedProcess) -> Result<(), Box<dyn Error>> {
    let mut stdin = io::stdin();
    let mut remote_stdin = attached_process
        .stdin()
        .expect("cannot obtain container's stdin");
    let mut stdout = io::stdout();
    let mut remote_stdout = attached_process
        .stdout()
        .expect("cannot obtain container's stdout");

    // Check that stdin is a terminal
    if let Err(_) = termios::tcgetsid(stdin.as_fd()) {
        return Err("stdin is not a terminal".into());
    }

    // Get current terminal's attributes and make a copy to restore them after remote session ends
    let term_prev_attr = make_terminal_raw(&stdin)?;

    // From this point on, if there is any error we need to restore the terminal's attributes
    let result = tokio::select! {
        res = io::copy(&mut stdin, &mut remote_stdin) => res.map_or_else(|e| Err(Into::<Box<dyn Error>>::into(e)), |_| Ok(())),
        res = io::copy(&mut remote_stdout, &mut stdout) => res.map_or_else(|e| Err(Into::<Box<dyn Error>>::into(e)), |_| Ok(())),
        _ = keep_terminal_size_updated(io::stdin(), attached_process.terminal_size()) => Ok(()),
        res = attached_process.join() => res.map_or_else(|e| Err(Into::<Box<dyn Error>>::into(e)), |_| Ok(())),
    };

    restore_term_attr(&stdin, &term_prev_attr)?;

    result
}

async fn keep_terminal_size_updated<Fd: AsRawFd>(
    stdin: Fd,
    channel: Option<Sender<TerminalSize>>,
) -> Result<(), Box<dyn Error>> {
    if channel.is_none() {
        return Err("cannot communication terminal size to server".into());
    }

    let mut channel = channel.expect("could not retreive channel to send terminal size");

    // Wrap the signal channel in a stream that aggregates event in one second batches.
    // With this, we'll call the API only once per second maximum
    let window_change_signal_stream = SignalStream::new(unix::signal(SignalKind::window_change())?)
        .chunks_timeout(50, Duration::from_secs(1));

    tokio::pin!(window_change_signal_stream);

    let mut terminal_size: TerminalSize;

    loop {
        terminal_size = get_terminal_size(stdin.as_raw_fd())?;
        channel.try_send(terminal_size)?;

        let _ = window_change_signal_stream.next().await;
    }
}

async fn get_client() -> Result<Client, kube::Error> {
    let client = Client::try_default()
        .await
        .expect("failed to init default kubernetes client");

    debug!("connecting to cluster...");

    match client.apiserver_version().await {
        Ok(version_info) => {
            debug!("connected to cluster - version info: {:?}", version_info);
            Ok(client)
        }
        Err(e) => Err(e),
    }
}

fn create_attach_params(
    mut attach_params: AttachParams,
    container: Option<String>,
) -> AttachParams {
    if let Some(container_name) = container {
        attach_params = attach_params.container(container_name);
    }

    attach_params
}

fn create_command(shell: &str) -> Vec<String> {
    if shell == DEFAULT_SHELL {
        vec![DEFAULT_SHELL.to_string()]
    } else {
        env::var("TERM").map_or_else(
            |_| vec![shell.to_string()],
            |i| {
                vec![
                    shell.to_string(),
                    "-c".to_string(),
                    format!("TERM={} {}", i, shell),
                ]
            },
        )
    }
}
