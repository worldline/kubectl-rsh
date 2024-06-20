# kubectl-rsh

A [kubectl](https://github.com/kubernetes/kubectl) extension to open a shell session to a container in a pod, akin to [`oc rsh`](https://github.com/openshift/oc).

## Installation

Get the appropriate binary from the latest release, and put it somewhere in your `PATH` so that `kubectl` can find it. More information on the [Extend kubectl with plugin](https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/) page.

## Usage

Once installed, it can be invoked using `kubectl rsh`:

```
Usage: kubectl-rsh [OPTIONS] <POD_NAME> [CONTAINER_NAME]

Arguments:
  <POD_NAME>        Pod name
  [CONTAINER_NAME]  Container name; defaults to first container in the pod

Options:
  -s, --shell <shell>          Path to the shell [default: /bin/sh]
  -n, --namespace <namespace>  Namespace; defaults to namespace in current context
  -h, --help                   Print help
  -V, --version                Print version
```

## Prerequisites

You need to be logged in on the target cluster; this pluggin will use the current context in your `~/.kube/config` file.

## Building

This project aims at staying small and simple; for the moment the build process is `cargo build`.

[`cargo-watch`](https://github.com/watchexec/cargo-watch) is used for local development; use `make pipeline` to use it.

## Contributing

All contributions are welcome, open an issue if you face any problem; open a PR if you want to make an improvement of spontaneously fix something.

## TODO List

- Integration with Krew
- Autocomplete, if possible (pod names, and integration with bash and zsh)
- Windows & Mac support

## Notes

While this could simply hand everying over to `kubectl exec -it <POD_NAME> -- /bin/sh`, it wouldn't have been half as fun.
