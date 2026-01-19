use std::path::Path;
use std::process::Command;

use thiserror::Error;
use unidiff::PatchSet;

#[derive(Debug, Error)]
pub enum JJError {
    #[error("jj command failed: {0}")]
    CommandFailed(String),
    #[error("failed to execute jj: {0}")]
    IoError(#[from] std::io::Error),
    #[error("failed to parse diff output: {0}")]
    DiffParseError(String),
}

/// Result of a jj commit operation.
#[derive(Debug)]
pub struct CommitResult {
    pub stdout: String,
    pub stderr: String,
}

/// Gets the diff for the current working copy as a PatchSet.
pub fn working_copy_diff() -> Result<PatchSet, JJError> {
    let output = Command::new("jj")
        .args(["diff", "--git"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(JJError::CommandFailed(stderr));
    }

    let diff_str = String::from_utf8_lossy(&output.stdout);
    let mut patch = PatchSet::new();
    patch
        .parse(&diff_str)
        .map_err(|e| JJError::DiffParseError(e.to_string()))?;

    Ok(patch)
}

/// Runs `jj commit` without a message, opening the default editor.
pub fn commit_interactive() -> Result<CommitResult, JJError> {
    let output = Command::new("jj")
        .args(["commit"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !output.success() {
        return Err(JJError::CommandFailed("jj commit failed".to_string()));
    }

    Ok(CommitResult {
        stdout: String::new(),
        stderr: String::new(),
    })
}

/// Checks if the current working copy revision is empty (has no changes).
pub fn is_working_copy_empty() -> Result<bool, JJError> {
    let output = Command::new("jj")
        .args(["diff"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(JJError::CommandFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().is_empty())
}

/// Runs `jj commit` with the given message.
pub fn commit(message: &str) -> Result<CommitResult, JJError> {
    let output = Command::new("jj")
        .args(["commit", "-m", message])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(JJError::CommandFailed(stderr));
    }

    Ok(CommitResult { stdout, stderr })
}

/// A single revision with its diff.
#[derive(Debug)]
pub struct RevisionDiff {
    pub change_id: String,
    pub description: String,
    pub patch: PatchSet,
}

impl RevisionDiff {
    /// Returns true if any added line in this revision contains the given pattern.
    pub fn has_added_line_containing(&self, pattern: &str) -> bool {
        for file in self.patch.files() {
            for hunk in file.hunks() {
                for line in hunk.lines() {
                    if line.is_added() && line.value.contains(pattern) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Returns all added lines containing the given pattern.
    pub fn find_added_lines_containing(&self, pattern: &str) -> Vec<String> {
        let mut matches = Vec::new();
        for file in self.patch.files() {
            for hunk in file.hunks() {
                for line in hunk.lines() {
                    if line.is_added() && line.value.contains(pattern) {
                        matches.push(line.value.clone());
                    }
                }
            }
        }
        matches
    }
}

/// Gets the change history for a file, including diffs.
///
/// Returns a list of revisions that modified the file, along with their diffs.
pub fn file_history(path: &Path) -> Result<Vec<RevisionDiff>, JJError> {
    // First get the list of revisions that modified this file
    let log_output = Command::new("jj")
        .args([
            "log",
            "--no-graph",
            "-T",
            r#"change_id ++ "\n" ++ description ++ "\n---END_REV---\n"#,
            "-r",
            "::",
            path.to_str().unwrap_or(""),
        ])
        .output()?;

    if !log_output.status.success() {
        let stderr = String::from_utf8_lossy(&log_output.stderr).to_string();
        return Err(JJError::CommandFailed(stderr));
    }

    let log_stdout = String::from_utf8_lossy(&log_output.stdout);
    let mut revisions = Vec::new();

    for chunk in log_stdout.split("---END_REV---") {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }

        let mut lines = chunk.lines();
        let change_id = lines.next().unwrap_or("").to_string();
        let description = lines.collect::<Vec<_>>().join("\n");

        if change_id.is_empty() {
            continue;
        }

        // Get the diff for this revision
        let diff_output = Command::new("jj")
            .args([
                "diff",
                "--git",
                "-r",
                &change_id,
                path.to_str().unwrap_or(""),
            ])
            .output()?;

        if !diff_output.status.success() {
            let stderr = String::from_utf8_lossy(&diff_output.stderr).to_string();
            return Err(JJError::CommandFailed(stderr));
        }

        let diff_str = String::from_utf8_lossy(&diff_output.stdout);
        let mut patch = PatchSet::new();
        patch
            .parse(&diff_str)
            .map_err(|e| JJError::DiffParseError(e.to_string()))?;

        revisions.push(RevisionDiff {
            change_id,
            description,
            patch,
        });
    }

    Ok(revisions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_diff() {
        let mut patch = PatchSet::new();
        let result = patch.parse("");
        assert!(result.is_ok());
        assert_eq!(patch.len(), 0);
    }

    #[test]
    fn test_parse_simple_diff() {
        let diff = r#"diff --git a/test.txt b/test.txt
new file mode 100644
index 0000000..e69de29
--- /dev/null
+++ b/test.txt
@@ -0,0 +1 @@
+hello world
"#;
        let mut patch = PatchSet::new();
        let result = patch.parse(diff);
        assert!(result.is_ok());
        assert_eq!(patch.len(), 1);
    }
}
