use crate::auth::{AuthService, LoginRequest, LoginTokenSource};
use crate::command::{CommandError, CommandOutcome, OutputFormat};
use crate::issue::{IssueListRequest, IssueService, IssueStateFilter, IssueViewRequest};
use crate::repo::{CloneTransport, RepoCloneRequest, RepoService, RepoViewRequest};

pub fn run(args: Vec<String>) -> Result<CommandOutcome, CommandError> {
    let Some((command, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing command"));
    };

    match command.as_str() {
        "auth" => run_auth(rest),
        "issue" => run_issue(rest),
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

fn run_issue(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing issue subcommand"));
    };

    let issue = IssueService::from_env();

    match subcommand.as_str() {
        "list" => issue.list(parse_issue_list_args(rest)?),
        "view" => issue.view(parse_issue_view_args(rest)?),
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

fn parse_positive_integer_flag(flag: &str, value: &str) -> Result<u32, CommandError> {
    let parsed = value.parse::<u32>().ok().filter(|candidate| *candidate > 0);

    parsed.ok_or_else(|| {
        CommandError::usage(format!(
            "invalid value for {flag}: expected a positive integer"
        ))
    })
}
