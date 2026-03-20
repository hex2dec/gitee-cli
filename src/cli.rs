use std::path::PathBuf;

use crate::auth::{AuthService, LoginRequest, LoginTokenSource};
use crate::command::{CommandError, CommandOutcome, OutputFormat};
use crate::gitee_api::PullRequestListFilters;
use crate::issue::{
    IssueCommentBodySource, IssueCommentRequest, IssueListRequest, IssueService, IssueStateFilter,
    IssueViewRequest,
};
use crate::pr::{
    PrCommentRequest, PrCreateRequest, PrListRequest, PrService, PrStatusRequest, PrTextSource,
    PrViewRequest,
};
use crate::repo::{CloneTransport, RepoCloneRequest, RepoService, RepoViewRequest};

pub fn run(args: Vec<String>) -> Result<CommandOutcome, CommandError> {
    let Some((command, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing command"));
    };

    match command.as_str() {
        "auth" => run_auth(rest),
        "issue" => run_issue(rest),
        "pr" => run_pr(rest),
        "repo" => run_repo(rest),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_auth(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing auth subcommand"));
    };

    let auth = AuthService::from_env();

    match subcommand.as_str() {
        "status" => auth.status(parse_output_format(rest)?),
        "login" => auth.login(parse_auth_login_args(rest)?),
        "logout" => auth.logout(parse_output_format(rest)?),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_issue(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing issue subcommand"));
    };

    let issue = IssueService::from_env();

    match subcommand.as_str() {
        "comment" => issue.comment(parse_issue_comment_args(rest)?),
        "list" => issue.list(parse_issue_list_args(rest)?),
        "view" => issue.view(parse_issue_view_args(rest)?),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_pr(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing pr subcommand"));
    };

    let pr = PrService::from_env();

    match subcommand.as_str() {
        "comment" => pr.comment(parse_pr_comment_args(rest)?),
        "create" => pr.create(parse_pr_create_args(rest)?),
        "list" => pr.list(parse_pr_list_args(rest)?),
        "status" => pr.status(parse_pr_status_args(rest)?),
        "view" => pr.view(parse_pr_view_args(rest)?),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_repo(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing repo subcommand"));
    };

    let repo = RepoService::from_env();

    match subcommand.as_str() {
        "clone" => repo.clone(parse_repo_clone_args(rest)?),
        "view" => repo.view(parse_repo_view_args(rest)?),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn parse_output_format(args: &[String]) -> Result<OutputFormat, CommandError> {
    let mut output = OutputFormat::Text;
    for arg in args {
        match arg.as_str() {
            "--json" => output = OutputFormat::Json,
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }
    Ok(output)
}

fn parse_auth_login_args(args: &[String]) -> Result<LoginRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut token: Option<LoginTokenSource> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--token" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --token"));
                };
                if token.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --token or --with-token",
                    ));
                }
                token = Some(LoginTokenSource::Flag(value.clone()));
                index += 2;
            }
            "--with-token" => {
                if token.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --token or --with-token",
                    ));
                }
                token = Some(LoginTokenSource::Stdin);
                index += 1;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    let Some(token_source) = token else {
        return Err(CommandError::usage(
            "login requires --token or --with-token",
        ));
    };

    Ok(LoginRequest {
        output,
        token_source,
    })
}

fn parse_issue_list_args(args: &[String]) -> Result<IssueListRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut state = IssueStateFilter::Open;
    let mut search = None;
    let mut page = 1;
    let mut per_page = 20;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            "--state" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --state"));
                };
                state = IssueStateFilter::parse(value)?;
                index += 2;
            }
            "--search" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --search"));
                };
                search = Some(value.clone());
                index += 2;
            }
            "--page" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --page"));
                };
                page = parse_positive_integer_flag("--page", value)?;
                index += 2;
            }
            "--per-page" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --per-page"));
                };
                per_page = parse_positive_integer_flag("--per-page", value)?;
                index += 2;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    Ok(IssueListRequest {
        output,
        repo,
        state,
        search,
        page,
        per_page,
    })
}

fn parse_issue_view_args(args: &[String]) -> Result<IssueViewRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut comments = false;
    let mut page = 1;
    let mut per_page = 20;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            "--comments" => {
                comments = true;
                index += 1;
            }
            "--page" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --page"));
                };
                page = parse_positive_integer_flag("--page", value)?;
                index += 2;
            }
            "--per-page" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --per-page"));
                };
                per_page = parse_positive_integer_flag("--per-page", value)?;
                index += 2;
            }
            value if value.starts_with("--") => {
                return Err(CommandError::usage("unsupported command"));
            }
            value => {
                positionals.push(value.to_string());
                index += 1;
            }
        }
    }

    let Some(number) = positionals.first() else {
        return Err(CommandError::usage("issue view requires an issue number"));
    };

    if positionals.len() > 1 {
        return Err(CommandError::usage(
            "issue view accepts exactly one issue number",
        ));
    }

    Ok(IssueViewRequest {
        output,
        repo,
        number: number.clone(),
        comments,
        page,
        per_page,
    })
}

fn parse_issue_comment_args(args: &[String]) -> Result<IssueCommentRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut body = None;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            "--body" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --body"));
                };
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body, --body-file, or --body-stdin",
                    ));
                }
                body = Some(IssueCommentBodySource::Flag(value.clone()));
                index += 2;
            }
            "--body-file" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --body-file"));
                };
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body, --body-file, or --body-stdin",
                    ));
                }
                body = Some(IssueCommentBodySource::File(PathBuf::from(value)));
                index += 2;
            }
            "--body-stdin" => {
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body, --body-file, or --body-stdin",
                    ));
                }
                body = Some(IssueCommentBodySource::Stdin);
                index += 1;
            }
            value if value.starts_with("--") => {
                return Err(CommandError::usage("unsupported command"));
            }
            value => {
                positionals.push(value.to_string());
                index += 1;
            }
        }
    }

    let Some(number) = positionals.first() else {
        return Err(CommandError::usage(
            "issue comment requires an issue number",
        ));
    };

    if positionals.len() > 1 {
        return Err(CommandError::usage(
            "issue comment accepts exactly one issue number",
        ));
    }

    let Some(body) = body else {
        return Err(CommandError::usage(
            "issue comment requires one of --body, --body-file, or --body-stdin",
        ));
    };

    Ok(IssueCommentRequest {
        output,
        repo,
        number: number.clone(),
        body,
    })
}

fn parse_pr_view_args(args: &[String]) -> Result<PrViewRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut number = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            value if value.starts_with("--") => {
                return Err(CommandError::usage("unsupported command"));
            }
            value => {
                if number.is_some() {
                    return Err(CommandError::usage(
                        "pr view accepts exactly one pull request number",
                    ));
                }

                let parsed = value.parse::<u64>().map_err(|_| {
                    CommandError::usage("invalid pull request number: expected a positive integer")
                })?;

                number = Some(parsed);
                index += 1;
            }
        }
    }

    let Some(number) = number else {
        return Err(CommandError::usage(
            "pr view requires a pull request number",
        ));
    };

    Ok(PrViewRequest {
        output,
        repo,
        number,
    })
}

fn parse_repo_view_args(args: &[String]) -> Result<RepoViewRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    Ok(RepoViewRequest { output, repo })
}

fn parse_repo_clone_args(args: &[String]) -> Result<RepoCloneRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut transport = CloneTransport::Https;
    let mut positionals = Vec::new();
    let mut transport_selected = false;

    for arg in args {
        match arg.as_str() {
            "--json" => output = OutputFormat::Json,
            "--https" => {
                if transport_selected {
                    return Err(CommandError::usage("provide only one of --https or --ssh"));
                }
                transport = CloneTransport::Https;
                transport_selected = true;
            }
            "--ssh" => {
                if transport_selected {
                    return Err(CommandError::usage("provide only one of --https or --ssh"));
                }
                transport = CloneTransport::Ssh;
                transport_selected = true;
            }
            value if value.starts_with("--") => {
                return Err(CommandError::usage("unsupported command"));
            }
            value => positionals.push(value.to_string()),
        }
    }

    let Some(repo) = positionals.first() else {
        return Err(CommandError::usage(
            "repo clone requires an owner/repo slug",
        ));
    };

    if positionals.len() > 2 {
        return Err(CommandError::usage(
            "repo clone accepts at most one destination path",
        ));
    }

    Ok(RepoCloneRequest {
        output,
        repo: repo.clone(),
        destination: positionals.get(1).cloned(),
        transport,
    })
}

fn parse_pr_comment_args(args: &[String]) -> Result<PrCommentRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut number = None;
    let mut body = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            "--body" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --body"));
                };
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body or --body-file",
                    ));
                }
                body = Some(PrTextSource::Inline(value.clone()));
                index += 2;
            }
            "--body-file" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --body-file"));
                };
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body or --body-file",
                    ));
                }
                body = Some(PrTextSource::File(value.clone()));
                index += 2;
            }
            value if value.starts_with("--") => {
                return Err(CommandError::usage("unsupported command"));
            }
            value => {
                if number.is_some() {
                    return Err(CommandError::usage(
                        "pr comment accepts exactly one pull request number",
                    ));
                }

                let parsed = value.parse::<u64>().map_err(|_| {
                    CommandError::usage("invalid pull request number: expected a positive integer")
                })?;

                number = Some(parsed);
                index += 1;
            }
        }
    }

    let Some(number) = number else {
        return Err(CommandError::usage(
            "pr comment requires a pull request number",
        ));
    };
    let Some(body) = body else {
        return Err(CommandError::usage(
            "pr comment requires --body or --body-file",
        ));
    };

    Ok(PrCommentRequest {
        output,
        repo,
        number,
        body,
    })
}
fn parse_pr_list_args(args: &[String]) -> Result<PrListRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut state = None;
    let mut author = None;
    let mut assignee = None;
    let mut base = None;
    let mut head = None;
    let mut limit = 30usize;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            "--state" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --state"));
                };
                state = Some(parse_pr_state(value)?);
                index += 2;
            }
            "--author" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --author"));
                };
                author = Some(value.clone());
                index += 2;
            }
            "--assignee" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --assignee"));
                };
                assignee = Some(value.clone());
                index += 2;
            }
            "--base" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --base"));
                };
                base = Some(value.clone());
                index += 2;
            }
            "--head" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --head"));
                };
                head = Some(value.clone());
                index += 2;
            }
            "--limit" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --limit"));
                };
                limit = parse_limit(value)?;
                index += 2;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    Ok(PrListRequest {
        output,
        repo,
        filters: PullRequestListFilters {
            state,
            author,
            assignee,
            base,
            head,
            limit,
        },
    })
}

fn parse_pr_create_args(args: &[String]) -> Result<PrCreateRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut repo = None;
    let mut head = None;
    let mut base = None;
    let mut title = None;
    let mut body = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --repo"));
                };
                repo = Some(value.clone());
                index += 2;
            }
            "--head" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --head"));
                };
                head = Some(value.clone());
                index += 2;
            }
            "--base" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --base"));
                };
                base = Some(value.clone());
                index += 2;
            }
            "--title" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --title"));
                };
                title = Some(value.clone());
                index += 2;
            }
            "--body" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --body"));
                };
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body or --body-file",
                    ));
                }
                body = Some(PrTextSource::Inline(value.clone()));
                index += 2;
            }
            "--body-file" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --body-file"));
                };
                if body.is_some() {
                    return Err(CommandError::usage(
                        "provide only one of --body or --body-file",
                    ));
                }
                body = Some(PrTextSource::File(value.clone()));
                index += 2;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    let Some(title) = title else {
        return Err(CommandError::usage("pr create requires --title"));
    };

    Ok(PrCreateRequest {
        output,
        repo,
        head,
        base,
        title,
        body,
    })
}

fn parse_pr_state(value: &str) -> Result<String, CommandError> {
    match value {
        "open" | "closed" | "merged" | "all" => Ok(value.to_string()),
        _ => Err(CommandError::usage(
            "invalid value for --state: expected open, closed, merged, or all",
        )),
    }
}

fn parse_limit(value: &str) -> Result<usize, CommandError> {
    let parsed = value.parse::<usize>().map_err(|_| {
        CommandError::usage("invalid value for --limit: expected a positive integer")
    })?;

    if parsed == 0 {
        return Err(CommandError::usage(
            "invalid value for --limit: expected a positive integer",
        ));
    }

    Ok(parsed)
}

fn parse_pr_status_args(args: &[String]) -> Result<PrStatusRequest, CommandError> {
    let mut output = OutputFormat::Text;
    let mut state = None;
    let mut limit = 30usize;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                output = OutputFormat::Json;
                index += 1;
            }
            "--state" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --state"));
                };
                state = Some(parse_pr_state(value)?);
                index += 2;
            }
            "--limit" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(CommandError::usage("missing value for --limit"));
                };
                limit = parse_limit(value)?;
                index += 2;
            }
            _ => return Err(CommandError::usage("unsupported command")),
        }
    }

    Ok(PrStatusRequest {
        output,
        filters: PullRequestListFilters {
            state,
            author: None,
            assignee: None,
            base: None,
            head: None,
            limit,
        },
    })
}

fn parse_positive_integer_flag(flag: &str, value: &str) -> Result<u32, CommandError> {
    let parsed = value.parse::<u32>().ok().filter(|candidate| *candidate > 0);

    parsed.ok_or_else(|| {
        CommandError::usage(format!(
            "invalid value for {flag}: expected a positive integer"
        ))
    })
}
