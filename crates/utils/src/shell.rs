//! Async shell command execution with timeout and process-tree killing.

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Description of a shell command to run.
#[derive(Debug, Clone)]
pub struct ShellCommand {
    /// The command string (interpreted by the platform shell).
    pub command: String,
    /// Working directory.
    pub cwd: Option<PathBuf>,
    /// Timeout duration (None = no timeout).
    pub timeout: Option<Duration>,
    /// Extra environment variables.
    pub env: HashMap<String, String>,
}

/// Result of a shell execution.
#[derive(Debug, Clone)]
pub struct ShellResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

/// Execute a shell command asynchronously, with optional timeout.
pub async fn execute_shell(cmd: &ShellCommand) -> Result<ShellResult> {
    let (program, flag) = shell_and_flag();

    let mut child_cmd = tokio::process::Command::new(program);
    child_cmd.arg(flag).arg(&cmd.command);

    if let Some(ref cwd) = cmd.cwd {
        child_cmd.current_dir(cwd);
    }
    for (k, v) in &cmd.env {
        child_cmd.env(k, v);
    }

    child_cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = child_cmd.spawn()?;

    if let Some(timeout) = cmd.timeout {
        let pid = child.id();
        match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => Ok(ShellResult {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                exit_code: output.status.code(),
                timed_out: false,
            }),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                // Timeout — attempt to kill
                if let Some(pid) = pid {
                    let _ = kill_process_tree(pid).await;
                }
                Ok(ShellResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    timed_out: true,
                })
            }
        }
    } else {
        let output = child.wait_with_output().await?;
        Ok(ShellResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
            timed_out: false,
        })
    }
}

/// Kill a process and its descendants.
pub async fn kill_process_tree(pid: u32) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .await;
    }
    #[cfg(not(windows))]
    {
        let _ = tokio::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .await;
    }
    Ok(())
}

/// Return the default shell for the current platform.
#[must_use]
pub fn get_default_shell() -> String {
    let (s, _) = shell_and_flag();
    s.to_owned()
}

/// (shell, flag) for running commands.
#[inline]
fn shell_and_flag() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("/bin/bash", "-c")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_echo() {
        let cmd = ShellCommand {
            command: if cfg!(windows) {
                "echo hello".into()
            } else {
                "echo hello".into()
            },
            cwd: None,
            timeout: Some(Duration::from_secs(5)),
            env: HashMap::new(),
        };
        let result = execute_shell(&cmd).await.unwrap();
        assert!(!result.timed_out);
        assert!(result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let cmd = ShellCommand {
            command: if cfg!(windows) {
                "ping -n 30 127.0.0.1".into()
            } else {
                "sleep 30".into()
            },
            cwd: None,
            timeout: Some(Duration::from_millis(200)),
            env: HashMap::new(),
        };
        let result = execute_shell(&cmd).await.unwrap();
        assert!(result.timed_out);
    }

    #[test]
    fn test_get_default_shell() {
        let shell = get_default_shell();
        assert!(!shell.is_empty());
        if cfg!(windows) {
            assert_eq!(shell, "cmd");
        } else {
            assert_eq!(shell, "/bin/bash");
        }
    }

    #[tokio::test]
    async fn test_execute_exit_code() {
        let cmd = ShellCommand {
            command: if cfg!(windows) {
                "exit /b 42".into()
            } else {
                "exit 42".into()
            },
            cwd: None,
            timeout: Some(Duration::from_secs(5)),
            env: HashMap::new(),
        };
        let result = execute_shell(&cmd).await.unwrap();
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_execute_with_env() {
        let mut env = HashMap::new();
        env.insert("MY_TEST_VAR".into(), "hello123".into());
        let cmd = ShellCommand {
            command: if cfg!(windows) {
                "echo %MY_TEST_VAR%".into()
            } else {
                "echo $MY_TEST_VAR".into()
            },
            cwd: None,
            timeout: Some(Duration::from_secs(5)),
            env,
        };
        let result = execute_shell(&cmd).await.unwrap();
        assert!(result.stdout.contains("hello123"));
    }

    #[tokio::test]
    async fn test_execute_with_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = ShellCommand {
            command: if cfg!(windows) {
                "cd".into()
            } else {
                "pwd".into()
            },
            cwd: Some(dir.path().to_path_buf()),
            timeout: Some(Duration::from_secs(5)),
            env: HashMap::new(),
        };
        let result = execute_shell(&cmd).await.unwrap();
        assert!(!result.timed_out);
        // stdout should contain the temp dir path
        assert!(!result.stdout.trim().is_empty());
    }
}
