mod auth;
mod client;
mod issue;
mod pull_request;
mod repo;
mod utils;

pub use auth::AuthError;
pub use client::GiteeClient;
pub use issue::{
    CreateIssue, IssueCommentResponse, IssueError, IssueListOptions, IssueResponse,
    IssueUserResponse,
};
pub use pull_request::{
    CreatePullRequest, CreatePullRequestComment, MergePullRequest, PullRequestBranchResponse,
    PullRequestCommentResponse, PullRequestError, PullRequestListFilters, PullRequestMergeResponse,
    PullRequestRepositoryResponse, PullRequestResponse, PullRequestUserResponse, UpdatePullRequest,
};
pub use repo::{RepoError, RepositoryResponse};
