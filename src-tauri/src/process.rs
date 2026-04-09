use crate::models::{LogEvent, LogKind, LogStream, RunLogEntry, RunRecord};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub const LOG_EVENT: &str = "deployment-log";
const MAX_RETAINED_LOG_LINES: usize = 2000;

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub run_id: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub current_dir: PathBuf,
    pub record: RunRecord,
}

#[derive(Clone)]
pub struct ProcessManager {
    active: Arc<Mutex<Option<ActiveProcess>>>,
}

#[derive(Clone)]
struct ActiveProcess {
    child: Arc<Mutex<Child>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn cancel(&self) -> Result<bool> {
        let active = self.active.lock().await.clone();
        if let Some(active) = active {
            active.child.lock().await.kill().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn run(&self, app: AppHandle, spec: CommandSpec) -> Result<RunRecord> {
        {
            let active = self.active.lock().await;
            if active.is_some() {
                return Err(anyhow!(
                    "A git-ftp process is already running. Cancel it before starting another deployment."
                ));
            }
        }

        let mut command = Command::new(&spec.executable);
        command
            .args(&spec.args)
            .current_dir(&spec.current_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());

        for (key, value) in &spec.envs {
            command.env(key, value);
        }

        let mut child = command.spawn().map_err(|error| {
            anyhow!(
                "Failed to start `{}`: {error}",
                spec.executable.to_string_lossy()
            )
        })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));

        {
            let mut active = self.active.lock().await;
            *active = Some(ActiveProcess {
                child: Arc::clone(&child),
            });
        }

        let logs = Arc::new(Mutex::new(Vec::<RunLogEntry>::new()));
        let record = Arc::new(Mutex::new(spec.record.clone()));

        emit_log(
            &app,
            &spec.run_id,
            &logs,
            LogStream::System,
            LogKind::System,
            format!("Starting {}", spec.record.command_preview),
        )
        .await;

        let stdout_task = stdout.map(|pipe| {
            let app = app.clone();
            let logs = Arc::clone(&logs);
            let run_id = spec.run_id.clone();
            tokio::spawn(async move {
                read_stream(app, run_id, logs, pipe, LogStream::Stdout).await;
            })
        });

        let stderr_task = stderr.map(|pipe| {
            let app = app.clone();
            let logs = Arc::clone(&logs);
            let run_id = spec.run_id.clone();
            tokio::spawn(async move {
                read_stream(app, run_id, logs, pipe, LogStream::Stderr).await;
            })
        });

        let status = child.lock().await.wait().await?;

        if let Some(task) = stdout_task {
            let _ = task.await;
        }
        if let Some(task) = stderr_task {
            let _ = task.await;
        }

        let mut final_record = record.lock().await.clone();
        final_record.finished_at = Some(crate::git_ftp::timestamp_now());
        final_record.exit_code = status.code();
        final_record.success = status.success();
        final_record.cancelled = !status.success() && status.code().is_none();
        final_record.logs = logs.lock().await.clone();

        emit_log(
            &app,
            &spec.run_id,
            &logs,
            LogStream::System,
            if final_record.success {
                LogKind::System
            } else {
                LogKind::Error
            },
            match (final_record.success, final_record.exit_code) {
                (true, Some(code)) => format!("Finished successfully with exit code {code}."),
                (true, None) => "Finished successfully.".to_string(),
                (false, Some(code)) => format!("Process exited with code {code}."),
                (false, None) => "Process was terminated before returning an exit code.".to_string(),
            },
        )
        .await;

        final_record.logs = logs.lock().await.clone();

        let mut active = self.active.lock().await;
        *active = None;

        Ok(final_record)
    }
}

async fn read_stream<R>(
    app: AppHandle,
    run_id: String,
    logs: Arc<Mutex<Vec<RunLogEntry>>>,
    pipe: R,
    stream: LogStream,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(pipe).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        let kind = classify_log(&stream, &line);
        emit_log(&app, &run_id, &logs, stream.clone(), kind, line).await;
    }
}

async fn emit_log(
    app: &AppHandle,
    run_id: &str,
    logs: &Arc<Mutex<Vec<RunLogEntry>>>,
    stream: LogStream,
    kind: LogKind,
    line: String,
) {
    let entry = RunLogEntry {
        timestamp: crate::git_ftp::timestamp_now(),
        stream,
        kind,
        line,
    };
    let mut guard = logs.lock().await;
    guard.push(entry.clone());
    if guard.len() > MAX_RETAINED_LOG_LINES {
        let excess = guard.len() - MAX_RETAINED_LOG_LINES;
        guard.drain(0..excess);
    }
    let _ = app.emit(
        LOG_EVENT,
        LogEvent {
            run_id: run_id.to_string(),
            entry,
        },
    );
}

fn classify_log(stream: &LogStream, line: &str) -> LogKind {
    let lowered = line.to_ascii_lowercase();
    if matches!(stream, LogStream::Stderr) || lowered.contains("error") || lowered.contains("failed")
    {
        LogKind::Error
    } else if lowered.contains("warning") {
        LogKind::Warning
    } else {
        LogKind::Info
    }
}
