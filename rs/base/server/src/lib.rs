use slog::{info, Logger};

/// Returns a `Future` that completes when the service should gracefully
/// shutdown. Completion happens if either of `SIGINT` or `SIGTERM` are
/// received.
#[cfg(unix)]
pub async fn shutdown_signal(log: Logger) {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sig_int =
        signal(SignalKind::interrupt()).expect("failed to install SIGINT signal handler");
    let mut sig_term =
        signal(SignalKind::terminate()).expect("failed to install SIGTERM signal handler");

    tokio::select! {
        _ = sig_int.recv() => {
            info!(log, "Caught SIGINT");
        }
        _ = sig_term.recv() => {
            info!(log, "Caught SIGTERM");
        }
    }
}

#[cfg(windows)]
pub async fn shutdown_signal(log: Logger) {
    use tokio::signal::windows::{ctrl_c, CtrlC};
    let mut sig_ctrl_c = ctrl_c().expect("failed to install CtrlC signal handler");
    sig_ctrl_c.recv().await;
}
