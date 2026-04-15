use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::auth::{AuthService, LoginRequest, LoginTokenSource};
use crate::command::{CommandError, CommandOutcome, EXIT_OK, OutputFormat};
use crate::gitee_api::PullRequestListFilters;
use crate::issue::{
    IssueBodySource, IssueCommentRequest, IssueCreateRequest, IssueListRequest, IssueService,
    IssueStateFilter, IssueViewRequest,
};
use crate::pr::{
    PrCheckoutRequest, PrCommentRequest, PrCreateRequest, PrEditRequest, PrListRequest, PrService,
    PrStatusRequest, PrTextSource, PrViewRequest,
};
use crate::repo::{CloneTransport, RepoCloneRequest, RepoService, RepoViewRequest};

enum ParseOutcome<T> {
    Value(T),
    Help(CommandOutcome),
}

const PR_DETAIL_JSON_FIELDS: &[&str] = &["number", "title", "url"];
const REPO_VIEW_JSON_FIELDS: &[&str] = &["name", "nameWithOwner", "url"];
const ISSUE_DETAIL_JSON_FIELDS: &[&str] = &["number", "title", "url"];

pub fn run(args: Vec<String>) -> Result<CommandOutcome, CommandError> {
    if matches!(args.as_slice(), [flag] if is_version_flag(flag)) {
        return Ok(render_version());
    }

    let Some((command, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing command"));
    };

    if is_help_flag(command) {
        return Ok(render_help(root_help_command()));
    }

    match command.as_str() {
        "auth" => run_auth(rest),
        "help" => run_help(rest),
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

    if is_help_flag(subcommand) {
        return Ok(render_help(auth_help_command()));
    }

    let auth = AuthService::from_env();

    match subcommand.as_str() {
        "status" => execute_parsed(parse_output_only(rest, auth_status_command()), |output| {
            auth.status(output)
        }),
        "login" => execute_parsed(parse_auth_login_args(rest), |request| auth.login(request)),
        "logout" => execute_parsed(parse_output_only(rest, auth_logout_command()), |output| {
            auth.logout(output)
        }),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_issue(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing issue subcommand"));
    };

    if is_help_flag(subcommand) {
        return Ok(render_help(issue_help_command()));
    }

    let issue = IssueService::from_env();

    match subcommand.as_str() {
        "create" => execute_parsed(parse_issue_create_args(rest), |request| {
            issue.create(request)
        }),
        "comment" => execute_parsed(parse_issue_comment_args(rest), |request| {
            issue.comment(request)
        }),
        "list" => execute_parsed(parse_issue_list_args(rest), |request| issue.list(request)),
        "view" => execute_parsed(parse_issue_view_args(rest), |request| issue.view(request)),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_pr(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing pr subcommand"));
    };

    if is_help_flag(subcommand) {
        return Ok(render_help(pr_help_command()));
    }

    let pr = PrService::from_env();

    match subcommand.as_str() {
        "checkout" => execute_parsed(parse_pr_checkout_args(rest), |request| pr.checkout(request)),
        "comment" => execute_parsed(parse_pr_comment_args(rest), |request| pr.comment(request)),
        "create" => execute_parsed(parse_pr_create_args(rest), |request| pr.create(request)),
        "edit" => execute_parsed(parse_pr_edit_args(rest), |request| pr.edit(request)),
        "list" => execute_parsed(parse_pr_list_args(rest), |request| pr.list(request)),
        "status" => execute_parsed(parse_pr_status_args(rest), |request| pr.status(request)),
        "view" => execute_parsed(parse_pr_view_args(rest), |request| pr.view(request)),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_repo(args: &[String]) -> Result<CommandOutcome, CommandError> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing repo subcommand"));
    };

    if is_help_flag(subcommand) {
        return Ok(render_help(repo_help_command()));
    }

    let repo = RepoService::from_env();

    match subcommand.as_str() {
        "clone" => execute_parsed(parse_repo_clone_args(rest), |request| repo.clone(request)),
        "view" => execute_parsed(parse_repo_view_args(rest), |request| repo.view(request)),
        _ => Err(CommandError::usage("unsupported command")),
    }
}

fn run_help(args: &[String]) -> Result<CommandOutcome, CommandError> {
    execute_parsed(parse_matches(help_command(), args), |matches| {
        let output = output_format(&matches);
        let topics = values(&matches, "topics");
        let Some(topic) = resolve_help_topic(&topics) else {
            return Err(CommandError::usage("unknown help topic"));
        };

        match output {
            OutputFormat::Text => Ok(render_help((topic.text_command)())),
            OutputFormat::Json { .. } => Ok(CommandOutcome::json(EXIT_OK, (topic.json)())),
        }
    })
}

struct HelpTopic {
    text_command: fn() -> Command,
    json: fn() -> serde_json::Value,
}

fn resolve_help_topic(path: &[String]) -> Option<HelpTopic> {
    let path = path.iter().map(String::as_str).collect::<Vec<_>>();

    match path.as_slice() {
        [] => Some(HelpTopic {
            text_command: root_help_command,
            json: root_help_json,
        }),
        ["auth"] => Some(HelpTopic {
            text_command: auth_help_command,
            json: auth_help_json,
        }),
        ["auth", "login"] => Some(HelpTopic {
            text_command: auth_login_command,
            json: auth_login_help_json,
        }),
        ["auth", "logout"] => Some(HelpTopic {
            text_command: auth_logout_command,
            json: auth_logout_help_json,
        }),
        ["auth", "status"] => Some(HelpTopic {
            text_command: auth_status_command,
            json: auth_status_help_json,
        }),
        ["issue"] => Some(HelpTopic {
            text_command: issue_help_command,
            json: issue_help_json,
        }),
        ["issue", "comment"] => Some(HelpTopic {
            text_command: issue_comment_command,
            json: issue_comment_help_json,
        }),
        ["issue", "create"] => Some(HelpTopic {
            text_command: issue_create_command,
            json: issue_create_help_json,
        }),
        ["issue", "list"] => Some(HelpTopic {
            text_command: issue_list_command,
            json: issue_list_help_json,
        }),
        ["issue", "view"] => Some(HelpTopic {
            text_command: issue_view_command,
            json: issue_view_help_json,
        }),
        ["pr"] => Some(HelpTopic {
            text_command: pr_help_command,
            json: pr_help_json,
        }),
        ["pr", "checkout"] => Some(HelpTopic {
            text_command: pr_checkout_command,
            json: pr_checkout_help_json,
        }),
        ["pr", "comment"] => Some(HelpTopic {
            text_command: pr_comment_command,
            json: pr_comment_help_json,
        }),
        ["pr", "create"] => Some(HelpTopic {
            text_command: pr_create_command,
            json: pr_create_help_json,
        }),
        ["pr", "edit"] => Some(HelpTopic {
            text_command: pr_edit_command,
            json: pr_edit_help_json,
        }),
        ["pr", "list"] => Some(HelpTopic {
            text_command: pr_list_command,
            json: pr_list_help_json,
        }),
        ["pr", "status"] => Some(HelpTopic {
            text_command: pr_status_command,
            json: pr_status_help_json,
        }),
        ["pr", "view"] => Some(HelpTopic {
            text_command: pr_view_command,
            json: pr_view_help_json,
        }),
        ["repo"] => Some(HelpTopic {
            text_command: repo_help_command,
            json: repo_help_json,
        }),
        ["repo", "clone"] => Some(HelpTopic {
            text_command: repo_clone_command,
            json: repo_clone_help_json,
        }),
        ["repo", "view"] => Some(HelpTopic {
            text_command: repo_view_command,
            json: repo_view_help_json,
        }),
        _ => None,
    }
}

fn execute_parsed<T>(
    parsed: Result<ParseOutcome<T>, CommandError>,
    handler: impl FnOnce(T) -> Result<CommandOutcome, CommandError>,
) -> Result<CommandOutcome, CommandError> {
    match parsed? {
        ParseOutcome::Value(value) => handler(value),
        ParseOutcome::Help(help) => Ok(help),
    }
}

fn parse_output_only(
    args: &[String],
    command: Command,
) -> Result<ParseOutcome<OutputFormat>, CommandError> {
    let command_name = command.get_name().to_string();
    map_parsed(parse_matches(command, args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, &command_name, &[])?;
        Ok(output)
    })
}

fn parse_auth_login_args(args: &[String]) -> Result<ParseOutcome<LoginRequest>, CommandError> {
    map_parsed(parse_matches(auth_login_command(), args), |matches| {
        let output = output_format(&matches);
        let token_values = values(&matches, "token");
        let with_token_count = flag_count(&matches, "with_token");

        if token_values.len() + with_token_count > 1 {
            return Err(CommandError::usage(
                "provide only one of --token or --with-token",
            ));
        }

        let Some(token_source) = token_values
            .last()
            .map(|value| LoginTokenSource::Flag(value.clone()))
            .or_else(|| (with_token_count == 1).then_some(LoginTokenSource::Stdin))
        else {
            return Err(CommandError::usage(
                "login requires --token or --with-token",
            ));
        };

        Ok(LoginRequest {
            output,
            token_source,
        })
    })
}

fn parse_issue_list_args(args: &[String]) -> Result<ParseOutcome<IssueListRequest>, CommandError> {
    map_parsed(parse_matches(issue_list_command(), args), |matches| {
        let output = output_format(&matches);
        let repo = last_value(&matches, "repo");
        let state = match last_value(&matches, "state") {
            Some(value) => IssueStateFilter::parse(&value)?,
            None => IssueStateFilter::Open,
        };
        let search = last_value(&matches, "search");
        let page = match last_value(&matches, "page") {
            Some(value) => parse_positive_integer_flag("--page", &value)?,
            None => 1,
        };
        let per_page = match last_value(&matches, "per_page") {
            Some(value) => parse_positive_integer_flag("--per-page", &value)?,
            None => 20,
        };

        Ok(IssueListRequest {
            output,
            repo,
            state,
            search,
            page,
            per_page,
        })
    })
}

fn parse_issue_view_args(args: &[String]) -> Result<ParseOutcome<IssueViewRequest>, CommandError> {
    map_parsed(parse_matches(issue_view_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "issue view", ISSUE_DETAIL_JSON_FIELDS)?;
        let repo = last_value(&matches, "repo");
        let comments = flag_count(&matches, "comments") > 0;
        let page = match last_value(&matches, "page") {
            Some(value) => parse_positive_integer_flag("--page", &value)?,
            None => 1,
        };
        let per_page = match last_value(&matches, "per_page") {
            Some(value) => parse_positive_integer_flag("--per-page", &value)?,
            None => 20,
        };
        let positionals = values(&matches, "positionals");

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
    })
}

fn parse_issue_comment_args(
    args: &[String],
) -> Result<ParseOutcome<IssueCommentRequest>, CommandError> {
    map_parsed(parse_matches(issue_comment_command(), args), |matches| {
        let output = output_format(&matches);
        let repo = last_value(&matches, "repo");
        let body_values = values(&matches, "body");
        let body_file_values = values(&matches, "body_file");
        let positionals = values(&matches, "positionals");

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

        if body_values.len() + body_file_values.len() > 1 {
            return Err(CommandError::usage(
                "provide only one of --body or --body-file",
            ));
        }

        let Some(body) = body_values
            .last()
            .map(|value| IssueBodySource::Inline(value.clone()))
            .or_else(|| {
                body_file_values
                    .last()
                    .map(|value| IssueBodySource::File(PathBuf::from(value)))
            })
        else {
            return Err(CommandError::usage(
                "issue comment requires --body or --body-file",
            ));
        };

        Ok(IssueCommentRequest {
            output,
            repo,
            number: number.clone(),
            body,
        })
    })
}

fn parse_issue_create_args(
    args: &[String],
) -> Result<ParseOutcome<IssueCreateRequest>, CommandError> {
    map_parsed(parse_matches(issue_create_command(), args), |matches| {
        let output = output_format(&matches);
        let repo = last_value(&matches, "repo");
        let title = last_value(&matches, "title");
        let body_values = values(&matches, "body");
        let body_file_values = values(&matches, "body_file");

        if body_values.len() + body_file_values.len() > 1 {
            return Err(CommandError::usage(
                "provide only one of --body or --body-file",
            ));
        }

        let Some(title) = title else {
            return Err(CommandError::usage("issue create requires --title"));
        };

        let body = body_values
            .last()
            .map(|value| IssueBodySource::Inline(value.clone()))
            .or_else(|| {
                body_file_values
                    .last()
                    .map(|value| IssueBodySource::File(PathBuf::from(value)))
            });

        Ok(IssueCreateRequest {
            output,
            repo,
            title,
            body,
        })
    })
}

fn parse_pr_view_args(args: &[String]) -> Result<ParseOutcome<PrViewRequest>, CommandError> {
    map_parsed(parse_matches(pr_view_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr view", PR_DETAIL_JSON_FIELDS)?;
        let repo = last_value(&matches, "repo");
        let positionals = values(&matches, "positionals");

        let Some(number) = positionals.first() else {
            return Err(CommandError::usage(
                "pr view requires a pull request number",
            ));
        };

        if positionals.len() > 1 {
            return Err(CommandError::usage(
                "pr view accepts exactly one pull request number",
            ));
        }

        let number = number.parse::<u64>().map_err(|_| {
            CommandError::usage("invalid pull request number: expected a positive integer")
        })?;

        Ok(PrViewRequest {
            output,
            repo,
            number,
        })
    })
}

fn parse_repo_view_args(args: &[String]) -> Result<ParseOutcome<RepoViewRequest>, CommandError> {
    map_parsed(parse_matches(repo_view_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "repo view", REPO_VIEW_JSON_FIELDS)?;
        Ok(RepoViewRequest {
            output,
            repo: last_value(&matches, "repo"),
        })
    })
}

fn parse_repo_clone_args(args: &[String]) -> Result<ParseOutcome<RepoCloneRequest>, CommandError> {
    map_parsed(parse_matches(repo_clone_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "repo clone", &[])?;
        let https_count = flag_count(&matches, "https");
        let ssh_count = flag_count(&matches, "ssh");
        let positionals = values(&matches, "positionals");

        if https_count + ssh_count > 1 {
            return Err(CommandError::usage("provide only one of --https or --ssh"));
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
            transport: if ssh_count == 1 {
                Some(CloneTransport::Ssh)
            } else if https_count == 1 {
                Some(CloneTransport::Https)
            } else {
                None
            },
        })
    })
}

fn parse_pr_comment_args(args: &[String]) -> Result<ParseOutcome<PrCommentRequest>, CommandError> {
    map_parsed(parse_matches(pr_comment_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr comment", &[])?;
        let repo = last_value(&matches, "repo");
        let body_values = values(&matches, "body");
        let body_file_values = values(&matches, "body_file");
        let positionals = values(&matches, "positionals");

        let Some(number) = positionals.first() else {
            return Err(CommandError::usage(
                "pr comment requires a pull request number",
            ));
        };

        if positionals.len() > 1 {
            return Err(CommandError::usage(
                "pr comment accepts exactly one pull request number",
            ));
        }

        if body_values.len() + body_file_values.len() > 1 {
            return Err(CommandError::usage(
                "provide only one of --body or --body-file",
            ));
        }

        let Some(body) = body_values
            .last()
            .map(|value| PrTextSource::Inline(value.clone()))
            .or_else(|| {
                body_file_values
                    .last()
                    .map(|value| PrTextSource::File(value.clone()))
            })
        else {
            return Err(CommandError::usage(
                "pr comment requires --body or --body-file",
            ));
        };

        let number = number.parse::<u64>().map_err(|_| {
            CommandError::usage("invalid pull request number: expected a positive integer")
        })?;

        Ok(PrCommentRequest {
            output,
            repo,
            number,
            body,
        })
    })
}

fn parse_pr_list_args(args: &[String]) -> Result<ParseOutcome<PrListRequest>, CommandError> {
    map_parsed(parse_matches(pr_list_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr list", &[])?;
        let repo = last_value(&matches, "repo");
        let state = match last_value(&matches, "state") {
            Some(value) => Some(parse_pr_state(&value)?),
            None => None,
        };
        let author = last_value(&matches, "author");
        let assignee = last_value(&matches, "assignee");
        let base = last_value(&matches, "base");
        let head = last_value(&matches, "head");
        let limit = match last_value(&matches, "limit") {
            Some(value) => parse_limit(&value)?,
            None => 30,
        };

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
    })
}

fn parse_pr_create_args(args: &[String]) -> Result<ParseOutcome<PrCreateRequest>, CommandError> {
    map_parsed(parse_matches(pr_create_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr create", PR_DETAIL_JSON_FIELDS)?;
        let repo = last_value(&matches, "repo");
        let head = last_value(&matches, "head");
        let base = last_value(&matches, "base");
        let title = last_value(&matches, "title");
        let body_values = values(&matches, "body");
        let body_file_values = values(&matches, "body_file");

        if body_values.len() + body_file_values.len() > 1 {
            return Err(CommandError::usage(
                "provide only one of --body or --body-file",
            ));
        }

        let Some(title) = title else {
            return Err(CommandError::usage("pr create requires --title"));
        };

        let body = body_values
            .last()
            .map(|value| PrTextSource::Inline(value.clone()))
            .or_else(|| {
                body_file_values
                    .last()
                    .map(|value| PrTextSource::File(value.clone()))
            });

        Ok(PrCreateRequest {
            output,
            repo,
            head,
            base,
            title,
            body,
        })
    })
}

fn parse_pr_edit_args(args: &[String]) -> Result<ParseOutcome<PrEditRequest>, CommandError> {
    map_parsed(parse_matches(pr_edit_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr edit", PR_DETAIL_JSON_FIELDS)?;
        let repo = last_value(&matches, "repo");
        let title = last_value(&matches, "title");
        let state = match last_value(&matches, "state") {
            Some(value) => Some(parse_pr_edit_state(&value)?),
            None => None,
        };
        let draft_count = flag_count(&matches, "draft");
        let ready_count = flag_count(&matches, "ready");
        let body_values = values(&matches, "body");
        let body_file_values = values(&matches, "body_file");
        let positionals = values(&matches, "positionals");

        let Some(number) = positionals.first() else {
            return Err(CommandError::usage(
                "pr edit requires a pull request number",
            ));
        };

        if positionals.len() > 1 {
            return Err(CommandError::usage(
                "pr edit accepts exactly one pull request number",
            ));
        }

        if body_values.len() + body_file_values.len() > 1 {
            return Err(CommandError::usage(
                "provide only one of --body or --body-file",
            ));
        }

        if draft_count + ready_count > 1 {
            return Err(CommandError::usage(
                "provide only one of --draft or --ready",
            ));
        }

        let body = body_values
            .last()
            .map(|value| PrTextSource::Inline(value.clone()))
            .or_else(|| {
                body_file_values
                    .last()
                    .map(|value| PrTextSource::File(value.clone()))
            });
        let draft = if draft_count == 1 {
            Some(true)
        } else if ready_count == 1 {
            Some(false)
        } else {
            None
        };

        if title.is_none() && body.is_none() && state.is_none() && draft.is_none() {
            return Err(CommandError::usage(
                "pr edit requires at least one of --title, --body, --body-file, --state, --draft, or --ready",
            ));
        }

        let number = number.parse::<u64>().map_err(|_| {
            CommandError::usage("invalid pull request number: expected a positive integer")
        })?;

        Ok(PrEditRequest {
            output,
            repo,
            number,
            title,
            body,
            state,
            draft,
        })
    })
}

fn parse_pr_checkout_args(
    args: &[String],
) -> Result<ParseOutcome<PrCheckoutRequest>, CommandError> {
    map_parsed(parse_matches(pr_checkout_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr checkout", &[])?;
        let repo = last_value(&matches, "repo");
        let positionals = values(&matches, "positionals");

        let Some(number) = positionals.first() else {
            return Err(CommandError::usage(
                "pr checkout requires a pull request number",
            ));
        };

        if positionals.len() > 1 {
            return Err(CommandError::usage(
                "pr checkout accepts exactly one pull request number",
            ));
        }

        let number = number.parse::<u64>().map_err(|_| {
            CommandError::usage("invalid pull request number: expected a positive integer")
        })?;

        Ok(PrCheckoutRequest {
            output,
            repo,
            number,
        })
    })
}

fn parse_pr_status_args(args: &[String]) -> Result<ParseOutcome<PrStatusRequest>, CommandError> {
    map_parsed(parse_matches(pr_status_command(), args), |matches| {
        let output = output_format(&matches);
        validate_json_field_selection(&output, "pr status", &[])?;
        let state = match last_value(&matches, "state") {
            Some(value) => Some(parse_pr_state(&value)?),
            None => None,
        };
        let limit = match last_value(&matches, "limit") {
            Some(value) => parse_limit(&value)?,
            None => 30,
        };

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
    })
}

fn map_parsed<T>(
    parsed: Result<ParseOutcome<ArgMatches>, CommandError>,
    mapper: impl FnOnce(ArgMatches) -> Result<T, CommandError>,
) -> Result<ParseOutcome<T>, CommandError> {
    match parsed? {
        ParseOutcome::Value(matches) => Ok(ParseOutcome::Value(mapper(matches)?)),
        ParseOutcome::Help(help) => Ok(ParseOutcome::Help(help)),
    }
}

fn parse_matches(
    command: Command,
    args: &[String],
) -> Result<ParseOutcome<ArgMatches>, CommandError> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push(command.get_name().to_string());
    argv.extend(args.iter().cloned());

    match command.try_get_matches_from(argv) {
        Ok(matches) => Ok(ParseOutcome::Value(matches)),
        Err(error) => match error.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                Ok(ParseOutcome::Help(CommandOutcome::text(
                    EXIT_OK,
                    error.to_string().trim_end().to_string(),
                )))
            }
            _ => Err(map_clap_error(error)),
        },
    }
}

fn map_clap_error(error: clap::Error) -> CommandError {
    if let Some(flag) = missing_value_flag(&error.to_string()) {
        return CommandError::usage(format!("missing value for {flag}"));
    }

    match error.kind() {
        clap::error::ErrorKind::UnknownArgument
        | clap::error::ErrorKind::InvalidSubcommand
        | clap::error::ErrorKind::ArgumentConflict
        | clap::error::ErrorKind::TooManyValues
        | clap::error::ErrorKind::WrongNumberOfValues
        | clap::error::ErrorKind::NoEquals
        | clap::error::ErrorKind::ValueValidation => CommandError::usage("unsupported command"),
        _ => CommandError::usage("unsupported command"),
    }
}

fn missing_value_flag(message: &str) -> Option<String> {
    let prefix = "a value is required for '";
    let start = message.find(prefix)? + prefix.len();
    let remaining = &message[start..];
    let end = remaining.find('\'')?;
    let argument = remaining[..end].split_whitespace().next()?;
    Some(argument.to_string())
}

fn output_format(matches: &ArgMatches) -> OutputFormat {
    let raw_json_values = values(matches, "json");
    let json_values = raw_json_values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if !raw_json_values.is_empty() {
        OutputFormat::Json {
            fields: (!json_values.is_empty()).then_some(json_values),
        }
    } else {
        OutputFormat::Text
    }
}

fn validate_json_field_selection(
    output: &OutputFormat,
    command_name: &str,
    supported_fields: &[&str],
) -> Result<(), CommandError> {
    let Some(fields) = output.json_fields() else {
        return Ok(());
    };

    if supported_fields.is_empty() {
        return Err(CommandError::usage(format!(
            "{command_name} does not support selecting JSON fields yet"
        )));
    }

    if let Some(field) = fields
        .iter()
        .find(|field| !supported_fields.contains(&field.as_str()))
    {
        return Err(CommandError::usage(format!(
            "unknown JSON field for {command_name}: {field}"
        )));
    }

    Ok(())
}

fn values(matches: &ArgMatches, id: &str) -> Vec<String> {
    matches
        .get_many::<String>(id)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

fn last_value(matches: &ArgMatches, id: &str) -> Option<String> {
    values(matches, id).into_iter().last()
}

fn flag_count(matches: &ArgMatches, id: &str) -> usize {
    usize::from(matches.get_count(id))
}

fn render_help(mut command: Command) -> CommandOutcome {
    let mut buffer = Vec::new();
    command
        .write_long_help(&mut buffer)
        .expect("writing clap help should succeed");

    CommandOutcome::text(
        EXIT_OK,
        String::from_utf8(buffer)
            .expect("clap help should be utf-8")
            .trim_end()
            .to_string(),
    )
}

fn render_version() -> CommandOutcome {
    CommandOutcome::text(EXIT_OK, format!("gitee {}", env!("CARGO_PKG_VERSION")))
}

fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "--help" | "-h")
}

fn is_version_flag(arg: &str) -> bool {
    matches!(arg, "--version" | "-V")
}

fn root_help_command() -> Command {
    base_command("gitee", "gitee")
        .about("Agent-first CLI for gitee.com")
        .arg(version_flag())
        .after_help(
            "Examples:\n  gitee auth status --json\n  gitee repo view --repo octo/demo --json\n  gitee help --json\n\nAgent discovery:\n  Use `gitee help --json` to inspect commands, flags, examples, and gh-style equivalents.",
        )
        .subcommand(auth_help_command())
        .subcommand(issue_help_command())
        .subcommand(pr_help_command())
        .subcommand(repo_help_command())
}

fn auth_help_command() -> Command {
    base_command("auth", "gitee auth")
        .about("Authenticate with gitee.com and inspect login state")
        .subcommand(auth_status_command())
        .subcommand(auth_login_command())
        .subcommand(auth_logout_command())
}

fn issue_help_command() -> Command {
    base_command("issue", "gitee issue")
        .about("Read, create, and comment on issues")
        .subcommand(issue_create_command())
        .subcommand(issue_comment_command())
        .subcommand(issue_list_command())
        .subcommand(issue_view_command())
}

fn pr_help_command() -> Command {
    base_command("pr", "gitee pr")
        .about("View, create, edit, comment on, and check out pull requests")
        .subcommand(pr_checkout_command())
        .subcommand(pr_comment_command())
        .subcommand(pr_create_command())
        .subcommand(pr_edit_command())
        .subcommand(pr_list_command())
        .subcommand(pr_status_command())
        .subcommand(pr_view_command())
}

fn repo_help_command() -> Command {
    base_command("repo", "gitee repo")
        .about("Inspect and clone repositories")
        .subcommand(repo_clone_command())
        .subcommand(repo_view_command())
}

fn auth_status_command() -> Command {
    output_only_command("status", "gitee auth status")
        .about("Check whether authentication is currently usable")
}

fn auth_login_command() -> Command {
    base_command("login", "gitee auth login")
        .about("Validate and save a personal access token")
        .arg(json_flag())
        .arg(string_option(
            "token",
            "token",
            "TOKEN",
            "Personal access token to validate and save",
        ))
        .arg(count_flag(
            "with_token",
            "with-token",
            "Read the token from stdin instead of a flag",
        ))
}

fn auth_logout_command() -> Command {
    output_only_command("logout", "gitee auth logout")
        .about("Remove the saved token from local config")
}

fn issue_list_command() -> Command {
    base_command("list", "gitee issue list")
        .about("List issues for a repository")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option(
            "state",
            "state",
            "STATE",
            "Filter issues by state: open, closed, or all",
        ))
        .arg(string_option(
            "search",
            "search",
            "SEARCH",
            "Filter issues by keyword text",
        ))
        .arg(string_option(
            "page",
            "page",
            "PAGE",
            "1-based page number to request",
        ))
        .arg(string_option(
            "per_page",
            "per-page",
            "PER_PAGE",
            "Number of issues to return per page",
        ))
}

fn issue_view_command() -> Command {
    base_command("view", "gitee issue view")
        .about("View a single issue and optionally include comments")
        .arg(json_flag())
        .arg(repo_option())
        .arg(count_flag(
            "comments",
            "comments",
            "Include issue comments in the response",
        ))
        .arg(string_option(
            "page",
            "page",
            "PAGE",
            "1-based page number for comment pagination",
        ))
        .arg(string_option(
            "per_page",
            "per-page",
            "PER_PAGE",
            "Number of comments to return per page",
        ))
        .arg(positionals_arg(
            "positionals",
            "ISSUE",
            "Issue number or identifier, such as I123",
        ))
}

fn issue_comment_command() -> Command {
    base_command("comment", "gitee issue comment")
        .about("Post a comment to an issue")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option(
            "body",
            "body",
            "BODY",
            "Inline comment body text",
        ))
        .arg(string_option(
            "body_file",
            "body-file",
            "PATH",
            "Read comment body text from a file",
        ))
        .arg(positionals_arg(
            "positionals",
            "ISSUE",
            "Issue number or identifier, such as I123",
        ))
}

fn issue_create_command() -> Command {
    base_command("create", "gitee issue create")
        .about("Create a new issue")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option("title", "title", "TITLE", "Issue title"))
        .arg(string_option(
            "body",
            "body",
            "BODY",
            "Inline issue body text",
        ))
        .arg(string_option(
            "body_file",
            "body-file",
            "PATH",
            "Read issue body text from a file",
        ))
}

fn pr_view_command() -> Command {
    base_command("view", "gitee pr view")
        .about("View a single pull request")
        .arg(json_flag())
        .arg(repo_option())
        .arg(positionals_arg("positionals", "PR", "Pull request number"))
}

fn repo_view_command() -> Command {
    output_only_command("view", "gitee repo view")
        .about("View repository metadata")
        .arg(repo_option())
}

fn repo_clone_command() -> Command {
    base_command("clone", "gitee repo clone")
        .about("Clone a repository by owner/repo slug")
        .arg(json_flag())
        .arg(count_flag("https", "https", "Clone over HTTPS"))
        .arg(count_flag("ssh", "ssh", "Clone over SSH"))
        .arg(positionals_arg(
            "positionals",
            "ARG",
            "Provide OWNER/REPO first, then an optional destination path",
        ))
}

fn pr_comment_command() -> Command {
    base_command("comment", "gitee pr comment")
        .about("Post a comment to a pull request")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option(
            "body",
            "body",
            "BODY",
            "Inline comment body text",
        ))
        .arg(string_option(
            "body_file",
            "body-file",
            "PATH",
            "Read comment body text from a file",
        ))
        .arg(positionals_arg("positionals", "PR", "Pull request number"))
}

fn pr_list_command() -> Command {
    base_command("list", "gitee pr list")
        .about("List pull requests with filters")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option(
            "state",
            "state",
            "STATE",
            "Filter pull requests by state: open, closed, merged, or all",
        ))
        .arg(string_option(
            "author",
            "author",
            "AUTHOR",
            "Filter pull requests by author login",
        ))
        .arg(string_option(
            "assignee",
            "assignee",
            "ASSIGNEE",
            "Filter pull requests by assignee login",
        ))
        .arg(string_option(
            "base",
            "base",
            "BASE",
            "Filter pull requests by base branch",
        ))
        .arg(string_option(
            "head",
            "head",
            "HEAD",
            "Filter pull requests by head branch",
        ))
        .arg(string_option(
            "limit",
            "limit",
            "LIMIT",
            "Maximum number of pull requests to return",
        ))
}

fn pr_create_command() -> Command {
    base_command("create", "gitee pr create")
        .about("Create a pull request from the current branch or an explicit head")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option(
            "head",
            "head",
            "HEAD",
            "Head branch to use instead of the current branch",
        ))
        .arg(string_option(
            "base",
            "base",
            "BASE",
            "Base branch to target",
        ))
        .arg(string_option(
            "title",
            "title",
            "TITLE",
            "Pull request title",
        ))
        .arg(string_option(
            "body",
            "body",
            "BODY",
            "Inline pull request body text",
        ))
        .arg(string_option(
            "body_file",
            "body-file",
            "PATH",
            "Read pull request body text from a file",
        ))
}

fn pr_edit_command() -> Command {
    base_command("edit", "gitee pr edit")
        .about("Edit an existing pull request")
        .arg(json_flag())
        .arg(repo_option())
        .arg(string_option(
            "title",
            "title",
            "TITLE",
            "Replace the pull request title",
        ))
        .arg(string_option(
            "body",
            "body",
            "BODY",
            "Replace the pull request body text",
        ))
        .arg(string_option(
            "body_file",
            "body-file",
            "PATH",
            "Read the pull request body text from a file",
        ))
        .arg(string_option(
            "state",
            "state",
            "STATE",
            "Change pull request state: open or closed",
        ))
        .arg(count_flag(
            "draft",
            "draft",
            "Mark the pull request as draft",
        ))
        .arg(count_flag(
            "ready",
            "ready",
            "Mark the pull request as ready",
        ))
        .arg(positionals_arg("positionals", "PR", "Pull request number"))
}

fn pr_checkout_command() -> Command {
    base_command("checkout", "gitee pr checkout")
        .about("Fetch and check out a pull request head branch")
        .arg(json_flag())
        .arg(repo_option())
        .arg(positionals_arg("positionals", "PR", "Pull request number"))
}

fn pr_status_command() -> Command {
    base_command("status", "gitee pr status")
        .about("Show pull requests related to the current local checkout")
        .arg(json_flag())
        .arg(string_option(
            "state",
            "state",
            "STATE",
            "Filter pull requests by state: open, closed, merged, or all",
        ))
        .arg(string_option(
            "limit",
            "limit",
            "LIMIT",
            "Maximum number of pull requests to return",
        ))
}

fn help_command() -> Command {
    base_command("help", "gitee help")
        .about("Show help for a command path or output machine-readable command metadata")
        .after_help(
            "Examples:\n  gitee help\n  gitee help --json\n  gitee help pr create\n  gitee help pr create --json",
        )
        .arg(json_flag())
        .arg(positionals_arg(
            "topics",
            "TOPIC",
            "Command path to inspect, such as `pr` or `pr create`",
        ))
}

fn output_only_command(name: &'static str, bin_name: &'static str) -> Command {
    base_command(name, bin_name).arg(json_flag())
}

fn base_command(name: &'static str, bin_name: &'static str) -> Command {
    Command::new(name)
        .bin_name(bin_name)
        .disable_version_flag(true)
}

fn json_flag() -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::Append)
        .num_args(0..=1)
        .default_missing_value("")
        .value_delimiter(',')
        .value_name("FIELDS")
        .help("Output machine-readable JSON")
}

fn version_flag() -> Arg {
    Arg::new("version")
        .short('V')
        .long("version")
        .action(ArgAction::SetTrue)
        .help("Print version")
}

fn count_flag(id: &'static str, long: &'static str, help: &'static str) -> Arg {
    Arg::new(id).long(long).action(ArgAction::Count).help(help)
}

fn string_option(
    id: &'static str,
    long: &'static str,
    value_name: &'static str,
    help: &'static str,
) -> Arg {
    Arg::new(id)
        .long(long)
        .action(ArgAction::Append)
        .num_args(1)
        .value_name(value_name)
        .allow_hyphen_values(true)
        .help(help)
}

fn repo_option() -> Arg {
    string_option(
        "repo",
        "repo",
        "REPO",
        "Target repository as OWNER/REPO; defaults to local git context when supported",
    )
}

fn positionals_arg(id: &'static str, value_name: &'static str, help: &'static str) -> Arg {
    Arg::new(id)
        .index(1)
        .action(ArgAction::Append)
        .num_args(0..)
        .value_name(value_name)
        .help(help)
}

fn root_help_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "root",
        "name": "gitee",
        "path": "gitee",
        "summary": "Agent-first CLI for gitee.com",
        "agent_guidance": {
            "recommended_discovery_command": "gitee help --json",
            "mental_model": "The command surface is intentionally similar to GitHub gh for auth, repo, issue, and pr workflows."
        },
        "unsupported_command_groups": [
            "api",
            "release",
            "label",
            "workflow",
            "notification"
        ],
        "commands": [
            auth_help_json(),
            issue_help_json(),
            pr_help_json(),
            repo_help_json()
        ]
    })
}

fn auth_help_json() -> serde_json::Value {
    help_group_json(
        "auth",
        "auth",
        "Authenticate with gitee.com and inspect login state",
        "gh auth",
        vec![
            auth_status_help_json(),
            auth_login_help_json(),
            auth_logout_help_json(),
        ],
    )
}

fn auth_status_help_json() -> serde_json::Value {
    help_command_json(
        "status",
        "auth status",
        "Check whether authentication is currently usable",
        "gh auth status",
        true,
        "not_required",
        false,
        false,
        false,
        vec![help_option_json(
            "--json",
            None,
            "Output machine-readable JSON",
            false,
        )],
        Vec::new(),
        Vec::new(),
        vec!["gitee auth status", "gitee auth status --json"],
        vec!["Reads the token from GITEE_TOKEN first, then the saved config file."],
    )
}

fn auth_login_help_json() -> serde_json::Value {
    help_command_json(
        "login",
        "auth login",
        "Validate and save a personal access token",
        "gh auth login",
        true,
        "not_required",
        false,
        false,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            help_option_json(
                "--token",
                Some("TOKEN"),
                "Personal access token to validate and save",
                false,
            ),
            help_option_json(
                "--with-token",
                None,
                "Read the token from stdin instead of a flag",
                false,
            ),
        ],
        Vec::new(),
        vec!["--token", "--with-token (stdin)"],
        vec![
            "gitee auth login --token \"$GITEE_TOKEN\" --json",
            "printf '%s\\n' \"$TOKEN\" | gitee auth login --with-token --json",
        ],
        vec!["Provide exactly one of --token or --with-token."],
    )
}

fn auth_logout_help_json() -> serde_json::Value {
    help_command_json(
        "logout",
        "auth logout",
        "Remove the saved token from local config",
        "gh auth logout",
        true,
        "not_required",
        false,
        false,
        false,
        vec![help_option_json(
            "--json",
            None,
            "Output machine-readable JSON",
            false,
        )],
        Vec::new(),
        Vec::new(),
        vec!["gitee auth logout --json"],
        vec!["Clears the saved config token but does not unset GITEE_TOKEN."],
    )
}

fn issue_help_json() -> serde_json::Value {
    help_group_json(
        "issue",
        "issue",
        "Read, create, and comment on issues",
        "gh issue",
        vec![
            issue_create_help_json(),
            issue_comment_help_json(),
            issue_list_help_json(),
            issue_view_help_json(),
        ],
    )
}

fn issue_list_help_json() -> serde_json::Value {
    help_command_json(
        "list",
        "issue list",
        "List issues for a repository",
        "gh issue list",
        true,
        "optional",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json(
                "--state",
                Some("STATE"),
                "Filter issues by state: open, closed, or all",
                false,
            ),
            help_option_json(
                "--search",
                Some("SEARCH"),
                "Filter issues by keyword text",
                false,
            ),
            help_option_json(
                "--page",
                Some("PAGE"),
                "1-based page number to request",
                false,
            ),
            help_option_json(
                "--per-page",
                Some("PER_PAGE"),
                "Number of issues to return per page",
                false,
            ),
        ],
        Vec::new(),
        Vec::new(),
        vec![
            "gitee issue list --repo octo/demo --state open --json",
            "gitee issue list --state open --page 1 --per-page 20 --json",
        ],
        vec![
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn issue_view_help_json() -> serde_json::Value {
    help_command_json(
        "view",
        "issue view",
        "View a single issue and optionally include comments",
        "gh issue view",
        true,
        "optional",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json(
                "--comments",
                None,
                "Include issue comments in the response",
                false,
            ),
            help_option_json(
                "--page",
                Some("PAGE"),
                "1-based page number for comment pagination",
                false,
            ),
            help_option_json(
                "--per-page",
                Some("PER_PAGE"),
                "Number of comments to return per page",
                false,
            ),
        ],
        vec![help_argument_json(
            "issue",
            "ISSUE",
            "Issue number or identifier, such as I123",
            true,
        )],
        Vec::new(),
        vec![
            "gitee issue view I123 --repo octo/demo --json",
            "gitee issue view I123 --comments --page 1 --per-page 20 --json",
        ],
        vec!["Comments are fetched only when --comments is provided."],
    )
}

fn issue_comment_help_json() -> serde_json::Value {
    help_command_json(
        "comment",
        "issue comment",
        "Post a comment to an issue",
        "gh issue comment",
        true,
        "required",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json("--body", Some("BODY"), "Inline comment body text", false),
            help_option_json(
                "--body-file",
                Some("PATH"),
                "Read comment body text from a file",
                false,
            ),
        ],
        vec![help_argument_json(
            "issue",
            "ISSUE",
            "Issue number or identifier, such as I123",
            true,
        )],
        vec!["--body", "--body-file"],
        vec![
            "gitee issue comment I123 --repo octo/demo --body \"Thanks for the report\" --json",
            "gitee issue comment I123 --body-file ./comment.md --json",
        ],
        vec![
            "Provide exactly one of --body or --body-file.",
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn issue_create_help_json() -> serde_json::Value {
    help_command_json(
        "create",
        "issue create",
        "Create a new issue",
        "gh issue create",
        true,
        "required",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json("--title", Some("TITLE"), "Issue title", true),
            help_option_json("--body", Some("BODY"), "Inline issue body text", false),
            help_option_json(
                "--body-file",
                Some("PATH"),
                "Read issue body text from a file",
                false,
            ),
        ],
        Vec::new(),
        vec!["--body", "--body-file"],
        vec![
            "gitee issue create --repo octo/demo --title \"New bug\" --body \"Steps to reproduce\" --json",
            "gitee issue create --title \"New bug\" --body-file ./issue.md --json",
        ],
        vec![
            "--title is required.",
            "Provide at most one of --body or --body-file.",
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_help_json() -> serde_json::Value {
    help_group_json(
        "pr",
        "pr",
        "View, create, edit, comment on, and check out pull requests",
        "gh pr",
        vec![
            pr_checkout_help_json(),
            pr_comment_help_json(),
            pr_create_help_json(),
            pr_edit_help_json(),
            pr_list_help_json(),
            pr_status_help_json(),
            pr_view_help_json(),
        ],
    )
}

fn pr_view_help_json() -> serde_json::Value {
    help_command_json(
        "view",
        "pr view",
        "View a single pull request",
        "gh pr view",
        true,
        "optional",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
        ],
        vec![help_argument_json("pr", "PR", "Pull request number", true)],
        Vec::new(),
        vec![
            "gitee pr view 42 --repo octo/demo --json",
            "gitee pr view 42 --json",
        ],
        vec![
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_comment_help_json() -> serde_json::Value {
    help_command_json(
        "comment",
        "pr comment",
        "Post a comment to a pull request",
        "gh pr comment",
        true,
        "required",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json("--body", Some("BODY"), "Inline comment body text", false),
            help_option_json(
                "--body-file",
                Some("PATH"),
                "Read comment body text from a file",
                false,
            ),
        ],
        vec![help_argument_json("pr", "PR", "Pull request number", true)],
        vec!["--body", "--body-file"],
        vec![
            "gitee pr comment 42 --repo octo/demo --body \"Ship it\" --json",
            "gitee pr comment 42 --body-file ./comment.md --json",
        ],
        vec![
            "Provide exactly one of --body or --body-file.",
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_create_help_json() -> serde_json::Value {
    help_command_json(
        "create",
        "pr create",
        "Create a pull request from the current branch or an explicit head",
        "gh pr create",
        true,
        "required",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json(
                "--head",
                Some("HEAD"),
                "Head branch to use instead of the current branch",
                false,
            ),
            help_option_json("--base", Some("BASE"), "Base branch to target", false),
            help_option_json("--title", Some("TITLE"), "Pull request title", true),
            help_option_json(
                "--body",
                Some("BODY"),
                "Inline pull request body text",
                false,
            ),
            help_option_json(
                "--body-file",
                Some("PATH"),
                "Read pull request body text from a file",
                false,
            ),
        ],
        Vec::new(),
        vec!["--body", "--body-file"],
        vec![
            "gitee pr create --title \"Use local head\" --base develop --body \"Built from the local branch\"",
            "gitee pr create --repo octo/demo --head feature/body-file --title \"Read body file\" --body-file ./body.md --json",
        ],
        vec![
            "--title is required.",
            "Provide at most one of --body or --body-file.",
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_edit_help_json() -> serde_json::Value {
    help_command_json(
        "edit",
        "pr edit",
        "Edit an existing pull request",
        "gh pr edit",
        true,
        "required",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json(
                "--title",
                Some("TITLE"),
                "Replace the pull request title",
                false,
            ),
            help_option_json(
                "--body",
                Some("BODY"),
                "Replace the pull request body text",
                false,
            ),
            help_option_json(
                "--body-file",
                Some("PATH"),
                "Read the pull request body text from a file",
                false,
            ),
            help_option_json(
                "--state",
                Some("STATE"),
                "Change pull request state: open or closed",
                false,
            ),
            help_option_json("--draft", None, "Mark the pull request as draft", false),
            help_option_json("--ready", None, "Mark the pull request as ready", false),
        ],
        vec![help_argument_json("pr", "PR", "Pull request number", true)],
        vec!["--body", "--body-file"],
        vec![
            "gitee pr edit 42 --repo octo/demo --title \"Updated title\" --json",
            "gitee pr edit 42 --body-file ./body.md --state open --ready --json",
        ],
        vec![
            "Provide at least one of --title, --body, --body-file, --state, --draft, or --ready.",
            "Provide at most one of --body or --body-file.",
            "Provide at most one of --draft or --ready.",
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_list_help_json() -> serde_json::Value {
    help_command_json(
        "list",
        "pr list",
        "List pull requests with filters",
        "gh pr list",
        true,
        "optional",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
            help_option_json(
                "--state",
                Some("STATE"),
                "Filter pull requests by state: open, closed, merged, or all",
                false,
            ),
            help_option_json(
                "--author",
                Some("AUTHOR"),
                "Filter pull requests by author login",
                false,
            ),
            help_option_json(
                "--assignee",
                Some("ASSIGNEE"),
                "Filter pull requests by assignee login",
                false,
            ),
            help_option_json(
                "--base",
                Some("BASE"),
                "Filter pull requests by base branch",
                false,
            ),
            help_option_json(
                "--head",
                Some("HEAD"),
                "Filter pull requests by head branch",
                false,
            ),
            help_option_json(
                "--limit",
                Some("LIMIT"),
                "Maximum number of pull requests to return",
                false,
            ),
        ],
        Vec::new(),
        Vec::new(),
        vec![
            "gitee pr list --repo octo/demo --state open --author octocat --limit 10 --json",
            "gitee pr list --state open --limit 10 --json",
        ],
        vec![
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_checkout_help_json() -> serde_json::Value {
    help_command_json(
        "checkout",
        "pr checkout",
        "Fetch and check out a pull request head branch",
        "gh pr checkout",
        true,
        "optional",
        true,
        true,
        true,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
        ],
        vec![help_argument_json("pr", "PR", "Pull request number", true)],
        Vec::new(),
        vec![
            "gitee pr checkout 42 --repo octo/demo --json",
            "gitee pr checkout 42 --json",
        ],
        vec![
            "Requires a local git checkout with an origin remote.",
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn pr_status_help_json() -> serde_json::Value {
    help_command_json(
        "status",
        "pr status",
        "Show pull requests related to the current local checkout",
        "gh pr status",
        true,
        "required",
        false,
        false,
        true,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            help_option_json(
                "--state",
                Some("STATE"),
                "Filter pull requests by state: open, closed, merged, or all",
                false,
            ),
            help_option_json(
                "--limit",
                Some("LIMIT"),
                "Maximum number of pull requests to return",
                false,
            ),
        ],
        Vec::new(),
        Vec::new(),
        vec![
            "gitee pr status --state open --limit 10 --json",
            "gitee pr status --json",
        ],
        vec!["Requires a local git checkout and authentication."],
    )
}

fn repo_help_json() -> serde_json::Value {
    help_group_json(
        "repo",
        "repo",
        "Inspect and clone repositories",
        "gh repo",
        vec![repo_clone_help_json(), repo_view_help_json()],
    )
}

fn repo_view_help_json() -> serde_json::Value {
    help_command_json(
        "view",
        "repo view",
        "View repository metadata",
        "gh repo view",
        true,
        "optional",
        true,
        true,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            repo_option_json(),
        ],
        Vec::new(),
        Vec::new(),
        vec![
            "gitee repo view --repo octo/demo --json",
            "gitee repo view --json",
        ],
        vec![
            "When --repo is omitted, the command can infer the repository from local git context.",
        ],
    )
}

fn repo_clone_help_json() -> serde_json::Value {
    help_command_json(
        "clone",
        "repo clone",
        "Clone a repository by owner/repo slug",
        "gh repo clone",
        true,
        "optional",
        false,
        false,
        false,
        vec![
            help_option_json("--json", None, "Output machine-readable JSON", false),
            help_option_json("--https", None, "Clone over HTTPS", false),
            help_option_json("--ssh", None, "Clone over SSH", false),
        ],
        vec![
            help_argument_json("repo", "OWNER/REPO", "Repository slug to clone", true),
            help_argument_json(
                "destination",
                "DESTINATION",
                "Optional local destination directory",
                false,
            ),
        ],
        Vec::new(),
        vec![
            "gitee repo clone octo/demo",
            "gitee repo clone octo/demo demo-https --https --json",
        ],
        vec![
            "Use at most one of --https or --ssh.",
            "When neither flag is provided, the CLI uses a saved clone protocol preference or prompts on first use.",
        ],
    )
}

fn help_group_json(
    name: &str,
    path: &str,
    summary: &str,
    gh_equivalent: &str,
    subcommands: Vec<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "group",
        "name": name,
        "path": path,
        "summary": summary,
        "gh_equivalent": gh_equivalent,
        "subcommands": subcommands
    })
}

#[allow(clippy::too_many_arguments)]
fn help_command_json(
    name: &str,
    path: &str,
    summary: &str,
    gh_equivalent: &str,
    supports_json: bool,
    auth: &str,
    repo_flag: bool,
    repo_inference: bool,
    local_git_required: bool,
    flags: Vec<serde_json::Value>,
    arguments: Vec<serde_json::Value>,
    input_sources: Vec<&str>,
    examples: Vec<&str>,
    notes: Vec<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "command",
        "name": name,
        "path": path,
        "summary": summary,
        "gh_equivalent": gh_equivalent,
        "supports_json": supports_json,
        "auth": auth,
        "repo_flag": repo_flag,
        "repo_inference": repo_inference,
        "local_git_required": local_git_required,
        "flags": flags,
        "arguments": arguments,
        "input_sources": input_sources,
        "examples": examples,
        "notes": notes
    })
}

fn repo_option_json() -> serde_json::Value {
    help_option_json(
        "--repo",
        Some("REPO"),
        "Target repository as OWNER/REPO; defaults to local git context when supported",
        false,
    )
}

fn help_option_json(
    name: &str,
    value_name: Option<&str>,
    description: &str,
    required: bool,
) -> serde_json::Value {
    match value_name {
        Some(value_name) => serde_json::json!({
            "kind": "option",
            "name": name,
            "value_name": value_name,
            "description": description,
            "required": required
        }),
        None => serde_json::json!({
            "kind": "option",
            "name": name,
            "description": description,
            "required": required
        }),
    }
}

fn help_argument_json(
    name: &str,
    value_name: &str,
    description: &str,
    required: bool,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "argument",
        "name": name,
        "value_name": value_name,
        "description": description,
        "required": required
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

fn parse_pr_edit_state(value: &str) -> Result<String, CommandError> {
    match value {
        "open" | "closed" => Ok(value.to_string()),
        _ => Err(CommandError::usage(
            "invalid value for --state: expected open or closed",
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

fn parse_positive_integer_flag(flag: &str, value: &str) -> Result<u32, CommandError> {
    let parsed = value.parse::<u32>().ok().filter(|candidate| *candidate > 0);

    parsed.ok_or_else(|| {
        CommandError::usage(format!(
            "invalid value for {flag}: expected a positive integer"
        ))
    })
}
