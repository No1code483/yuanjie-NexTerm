use git2::{Repository, Status, StatusOptions, DiffOptions, BranchType, Sort, Signature};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git 仓库未找到: {0}")]
    RepoNotFound(String),
    #[error("Git 操作失败: {0}")]
    GitError(String),
    #[error("IO 错误: {0}")]
    IoError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,       // M, A, D, U, R
    pub staged: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitStatusResult {
    pub branch: String,
    pub files: Vec<GitFileStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitBranchesResult {
    pub branches: Vec<String>,
    pub current: String,
}

pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        GitService
    }

    fn open_repo(path: &str) -> Result<Repository, GitError> {
        Repository::open(path).map_err(|e| GitError::RepoNotFound(format!("{}: {}", path, e)))
    }

    pub fn get_repo(&self, path: &str) -> Result<(), GitError> {
        Self::open_repo(path)?;
        Ok(())
    }

    pub fn get_status(&self, path: &str) -> Result<GitStatusResult, GitError> {
        let repo = Self::open_repo(path)?;

        let branch = Self::current_branch_name(&repo)?;

        let mut options = StatusOptions::new();
        options.include_untracked(true).include_ignored(false);

        let statuses = repo.statuses(Some(&mut options))
            .map_err(|e| GitError::GitError(format!("获取状态失败: {}", e)))?;

        let mut files = Vec::new();

        for entry in statuses.iter() {
            let file_path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            let status_char = if status.contains(Status::WT_NEW) || status.contains(Status::INDEX_NEW) {
                "A"
            } else if status.contains(Status::WT_DELETED) || status.contains(Status::INDEX_DELETED) {
                "D"
            } else if status.contains(Status::WT_MODIFIED) || status.contains(Status::INDEX_MODIFIED) {
                "M"
            } else if status.contains(Status::WT_RENAMED) || status.contains(Status::INDEX_RENAMED) {
                "R"
            } else {
                "U"
            };

            // Check if staged (in index)
            let staged = status.contains(Status::INDEX_NEW)
                || status.contains(Status::INDEX_MODIFIED)
                || status.contains(Status::INDEX_DELETED)
                || status.contains(Status::INDEX_RENAMED);

            // If it's both staged and unstaged, add both entries
            let has_wt = status.contains(Status::WT_NEW)
                || status.contains(Status::WT_MODIFIED)
                || status.contains(Status::WT_DELETED);

            if staged {
                files.push(GitFileStatus {
                    path: file_path.clone(),
                    status: status_char.to_string(),
                    staged: true,
                });
            }
            if has_wt {
                files.push(GitFileStatus {
                    path: file_path.clone(),
                    status: status_char.to_string(),
                    staged: false,
                });
            }
            if !staged && !has_wt {
                files.push(GitFileStatus {
                    path: file_path.clone(),
                    status: status_char.to_string(),
                    staged: false,
                });
            }
        }

        Ok(GitStatusResult { branch, files })
    }

    pub fn get_diff_file(&self, path: &str, file_path: &str) -> Result<String, GitError> {
        let repo = Self::open_repo(path)?;

        let tree = Self::head_tree(&repo)?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(file_path);

        let diff = repo.diff_tree_to_workdir_with_index(
            tree.as_ref(),
            Some(&mut diff_opts),
        ).map_err(|e| GitError::GitError(format!("获取差异失败: {}", e)))?;

        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            diff_text.push_str(&format!("{}{}", prefix, content));
            true
        }).map_err(|e| GitError::GitError(format!("打印差异失败: {}", e)))?;

        Ok(diff_text)
    }

    pub fn get_diff_unstaged(&self, path: &str) -> Result<String, GitError> {
        let repo = Self::open_repo(path)?;

        let tree = Self::head_tree(&repo)?;

        let diff = repo.diff_tree_to_workdir_with_index(
            tree.as_ref(),
            None,
        ).map_err(|e| GitError::GitError(format!("获取差异失败: {}", e)))?;

        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            diff_text.push_str(&format!("{}{}", prefix, content));
            true
        }).map_err(|e| GitError::GitError(format!("打印差异失败: {}", e)))?;

        Ok(diff_text)
    }

    pub fn stage_file(&self, path: &str, file_path: &str) -> Result<(), GitError> {
        let repo = Self::open_repo(path)?;
        let mut index = repo.index().map_err(|e| GitError::GitError(format!("获取索引失败: {}", e)))?;
        index.add_path(std::path::Path::new(file_path))
            .map_err(|e| GitError::GitError(format!("暂存文件失败: {}", e)))?;
        index.write().map_err(|e| GitError::GitError(format!("写入索引失败: {}", e)))?;
        Ok(())
    }

    pub fn stage_all(&self, path: &str) -> Result<(), GitError> {
        let repo = Self::open_repo(path)?;
        let mut index = repo.index().map_err(|e| GitError::GitError(format!("获取索引失败: {}", e)))?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| GitError::GitError(format!("暂存所有文件失败: {}", e)))?;
        index.write().map_err(|e| GitError::GitError(format!("写入索引失败: {}", e)))?;
        Ok(())
    }

    pub fn unstage_file(&self, path: &str, file_path: &str) -> Result<(), GitError> {
        let repo = Self::open_repo(path)?;
        let head = repo.head().map_err(|e| GitError::GitError(format!("获取 HEAD 失败: {}", e)))?;
        let head_commit = head.peel_to_commit().map_err(|e| GitError::GitError(format!("解析 HEAD 失败: {}", e)))?;
        let _head_tree = head_commit.tree().map_err(|e| GitError::GitError(format!("获取 HEAD 树失败: {}", e)))?;

        let mut index = repo.index().map_err(|e| GitError::GitError(format!("获取索引失败: {}", e)))?;
        index.remove_path(std::path::Path::new(file_path))
            .map_err(|e| GitError::GitError(format!("取消暂存文件失败: {}", e)))?;
        index.write().map_err(|e| GitError::GitError(format!("写入索引失败: {}", e)))?;

        // Also reset the file in index to HEAD
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.path(file_path).force();
        repo.checkout_index(Some(&mut index), Some(&mut checkout_opts))
            .map_err(|e| GitError::GitError(format!("重置文件失败: {}", e)))?;

        Ok(())
    }

    pub fn commit(&self, path: &str, message: &str) -> Result<String, GitError> {
        let repo = Self::open_repo(path)?;

        let signature = Signature::now("NexTerm", "nexterm@local")
            .map_err(|e| GitError::GitError(format!("创建签名失败: {}", e)))?;

        let mut index = repo.index().map_err(|e| GitError::GitError(format!("获取索引失败: {}", e)))?;
        let tree_oid = index.write_tree()
            .map_err(|e| GitError::GitError(format!("写入树失败: {}", e)))?;
        let tree = repo.find_tree(tree_oid)
            .map_err(|e| GitError::GitError(format!("查找树失败: {}", e)))?;

        let parent = match repo.head() {
            Ok(head) => {
                let head_commit = head.peel_to_commit()
                    .map_err(|e| GitError::GitError(format!("解析 HEAD 失败: {}", e)))?;
                vec![head_commit]
            }
            Err(_) => vec![],
        };

        let parent_refs: Vec<&git2::Commit> = parent.iter().collect();

        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        ).map_err(|e| GitError::GitError(format!("提交失败: {}", e)))?;

        Ok(oid.to_string())
    }

    pub fn push(&self, path: &str, remote: &str, branch: &str) -> Result<(), GitError> {
        let repo = Self::open_repo(path)?;

        let mut remote = repo.find_remote(remote)
            .map_err(|e| GitError::GitError(format!("查找远程仓库失败: {}", e)))?;

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

        let mut push_opts = git2::PushOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            git2::Cred::default()
        });
        push_opts.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| GitError::GitError(format!("推送失败: {}", e)))?;

        Ok(())
    }

    pub fn pull(&self, path: &str, remote: &str, branch: &str) -> Result<(), GitError> {
        let repo = Self::open_repo(path)?;

        let mut remote = repo.find_remote(remote)
            .map_err(|e| GitError::GitError(format!("查找远程仓库失败: {}", e)))?;

        let mut fetch_opts = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            git2::Cred::default()
        });
        fetch_opts.remote_callbacks(callbacks);

        remote.fetch(&[branch], Some(&mut fetch_opts), None)
            .map_err(|e| GitError::GitError(format!("获取远程分支失败: {}", e)))?;

        let fetch_head = repo.find_reference("FETCH_HEAD")
            .map_err(|e| GitError::GitError(format!("查找 FETCH_HEAD 失败: {}", e)))?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)
            .map_err(|e| GitError::GitError(format!("解析 FETCH_HEAD 失败: {}", e)))?;

        let (analysis, _) = repo.merge_analysis(&[&fetch_commit])
            .map_err(|e| GitError::GitError(format!("合并分析失败: {}", e)))?;

        if analysis.is_fast_forward() {
            let mut ref_to_update = repo.find_reference("HEAD")
                .map_err(|e| GitError::GitError(format!("获取 HEAD 失败: {}", e)))?;
            let ref_name = ref_to_update.name().unwrap_or("HEAD").to_string();

            ref_to_update.set_target(fetch_commit.id(), &format!("pull: Fast-forward"))
                .map_err(|e| GitError::GitError(format!("快进合并失败: {}", e)))?;

            repo.set_head(&ref_name)
                .map_err(|e| GitError::GitError(format!("设置 HEAD 失败: {}", e)))?;

            let mut checkout_opts = git2::build::CheckoutBuilder::new();
            checkout_opts.force();
            repo.checkout_head(Some(&mut checkout_opts))
                .map_err(|e| GitError::GitError(format!("检出失败: {}", e)))?;
        } else if analysis.is_normal() {
            return Err(GitError::GitError("需要手动合并，存在冲突".into()));
        }

        Ok(())
    }

    pub fn get_branches(&self, path: &str) -> Result<GitBranchesResult, GitError> {
        let repo = Self::open_repo(path)?;

        let current = Self::current_branch_name(&repo)?;

        let branches = repo.branches(Some(BranchType::Local))
            .map_err(|e| GitError::GitError(format!("获取分支列表失败: {}", e)))?;

        let mut branch_names = Vec::new();
        for branch_result in branches {
            let (branch, _) = branch_result
                .map_err(|e| GitError::GitError(format!("获取分支信息失败: {}", e)))?;
            let name = branch.name()
                .map_err(|e| GitError::GitError(format!("获取分支名失败: {}", e)))?;
            if let Some(name) = name {
                branch_names.push(name.to_string());
            }
        }

        Ok(GitBranchesResult {
            branches: branch_names,
            current,
        })
    }

    pub fn checkout_branch(&self, path: &str, branch: &str) -> Result<(), GitError> {
        let repo = Self::open_repo(path)?;

        let branch_ref = format!("refs/heads/{}", branch);
        let (object, reference) = repo.revparse_ext(&branch_ref)
            .map_err(|e| GitError::GitError(format!("解析分支引用失败: {}", e)))?;

        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.force();

        repo.checkout_tree(&object, Some(&mut checkout_opts))
            .map_err(|e| GitError::GitError(format!("检出树失败: {}", e)))?;

        match reference {
            Some(gref) => repo.set_head(gref.name().unwrap_or(&branch_ref))
                .map_err(|e| GitError::GitError(format!("设置 HEAD 失败: {}", e)))?,
            None => repo.set_head(&branch_ref)
                .map_err(|e| GitError::GitError(format!("设置 HEAD 失败: {}", e)))?,
        }

        Ok(())
    }

    pub fn get_log(&self, path: &str, count: u32) -> Result<Vec<GitCommit>, GitError> {
        let repo = Self::open_repo(path)?;

        let mut revwalk = repo.revwalk()
            .map_err(|e| GitError::GitError(format!("创建 revwalk 失败: {}", e)))?;
        revwalk.push_head()
            .map_err(|e| GitError::GitError(format!("推送 HEAD 失败: {}", e)))?;
        revwalk.set_sorting(Sort::TIME)
            .map_err(|e| GitError::GitError(format!("设置排序失败: {}", e)))?;

        let mut commits = Vec::new();
        for oid in revwalk.take(count as usize) {
            let oid = oid.map_err(|e| GitError::GitError(format!("遍历提交失败: {}", e)))?;
            let commit = repo.find_commit(oid)
                .map_err(|e| GitError::GitError(format!("查找提交失败: {}", e)))?;

            let hash = oid.to_string();
            let message = commit.message().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("Unknown").to_string();
            let timestamp = commit.time().seconds();

            commits.push(GitCommit {
                hash,
                message,
                author,
                timestamp,
            });
        }

        Ok(commits)
    }

    pub fn init_repo(&self, path: &str) -> Result<(), GitError> {
        Repository::init(path)
            .map_err(|e| GitError::GitError(format!("初始化仓库失败: {}", e)))?;
        Ok(())
    }

    fn head_tree(repo: &Repository) -> Result<Option<git2::Tree<'_>>, GitError> {
        match repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit()
                    .map_err(|e| GitError::GitError(format!("解析 HEAD 失败: {}", e)))?;
                let tree = commit.tree()
                    .map_err(|e| GitError::GitError(format!("获取 HEAD 树失败: {}", e)))?;
                Ok(Some(tree))
            }
            Err(_) => Ok(None),
        }
    }

    fn current_branch_name(repo: &Repository) -> Result<String, GitError> {
        if repo.head().is_err() {
            return Ok("(no commits)".to_string());
        }

        let head = repo.head().map_err(|e| GitError::GitError(format!("获取 HEAD 失败: {}", e)))?;

        if head.is_branch() {
            let name = head.shorthand().unwrap_or("unknown").to_string();
            Ok(name)
        } else {
            let oid = head.target()
                .map(|oid| oid.to_string().chars().take(7).collect::<String>())
                .unwrap_or_else(|| "?".to_string());
            Ok(format!("({})", oid))
        }
    }
}