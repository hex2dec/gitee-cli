mod auth;
mod client;
mod issue;
mod pull_request;
mod repo;
mod utils;

pub use auth::AuthError;
pub use client::GiteeClient;
pub use issue::{CreateIssue, Issue, IssueComment, IssueError, IssueListOptions};
pub use pull_request::{
    CreatePullRequest, CreatePullRequestComment, MergePullRequest, PullRequest, PullRequestBranch,
    PullRequestComment, PullRequestError, PullRequestListFilters, PullRequestMergeResult,
    UpdatePullRequest,
};
pub use repo::{RepoError, Repository};
