use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::auth::{AuthService, LoginRequest, LoginTokenSource};
use crate::command::{CommandError, CommandOutcome, EXIT_OK, OutputFormat};
use crate::gitee_api::PullRequestListFilters;
use crate::issue::{
    IssueCommentBodySource, IssueCommentRequest, IssueListRequest, IssueService, IssueStateFilter,
    IssueViewRequest,
};
use crate::pr::{
    PrCheckoutRequest, PrCommentRequest, PrCreateRequest, PrListRequest, PrService,
    PrStatusRequest, PrTextSource, PrViewRequest,
};
use crate::repo::{CloneTransport, RepoCloneRequest, RepoService, RepoViewRequest};

enum ParseOutcome<T> {
    Value(T),
    Help(CommandOutcome),
}

pub fn run(args: Vec<String>) -> Result<CommandOutcome, CommandError> {
    let Some((command, rest)) = args.split_first() else {
        return Err(CommandError::usage("missing command"));
    };

    if is_help_flag(command) {
        return Ok(render_help(root_help_command()));
    }

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
    map_parsed(parse_matches(command, args), |matches| {
        Ok(output_format(&matches))
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
            .map(|value| IssueCommentBodySource::Flag(value.clone()))
            .or_else(|| {
                body_file_values
                    .last()
                    .map(|value| IssueCommentBodySource::File(PathBuf::from(value)))
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

fn parse_pr_view_args(args: &[String]) -> Result<ParseOutcome<PrViewRequest>, CommandError> {
    map_parsed(parse_matches(pr_view_command(), args), |matches| {
        let output = output_format(&matches);
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
        Ok(RepoViewRequest {
            output: output_format(&matches),
            repo: last_value(&matches, "repo"),
        })
    })
}

fn parse_repo_clone_args(args: &[String]) -> Result<ParseOutcome<RepoCloneRequest>, CommandError> {
    map_parsed(parse_matches(repo_clone_command(), args), |matches| {
        let output = output_format(&matches);
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
                CloneTransport::Ssh
            } else {
                CloneTransport::Https
            },
        })
    })
}

fn parse_pr_comment_args(args: &[String]) -> Result<ParseOutcome<PrCommentRequest>, CommandError> {
    map_parsed(parse_matches(pr_comment_command(), args), |matches| {
        let output = output_format(&matches);
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

fn parse_pr_checkout_args(
    args: &[String],
) -> Result<ParseOutcome<PrCheckoutRequest>, CommandError> {
    map_parsed(parse_matches(pr_checkout_command(), args), |matches| {
        let output = output_format(&matches);
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
    if flag_count(matches, "json") > 0 {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
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

fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "--help" | "-h")
}

fn root_help_command() -> Command {
    base_command("gitee")
        .subcommand(auth_help_command())
        .subcommand(issue_help_command())
        .subcommand(pr_help_command())
        .subcommand(repo_help_command())
}

fn auth_help_command() -> Command {
    base_command("auth")
        .subcommand(auth_status_command())
        .subcommand(auth_login_command())
        .subcommand(auth_logout_command())
}

fn issue_help_command() -> Command {
    base_command("issue")
        .subcommand(issue_comment_command())
        .subcommand(issue_list_command())
        .subcommand(issue_view_command())
}

fn pr_help_command() -> Command {
    base_command("pr")
        .subcommand(pr_checkout_command())
        .subcommand(pr_comment_command())
        .subcommand(pr_create_command())
        .subcommand(pr_list_command())
        .subcommand(pr_status_command())
        .subcommand(pr_view_command())
}

fn repo_help_command() -> Command {
    base_command("repo")
        .subcommand(repo_clone_command())
        .subcommand(repo_view_command())
}

fn auth_status_command() -> Command {
    output_only_command("status")
}

fn auth_login_command() -> Command {
    base_command("login")
        .arg(json_flag())
        .arg(string_option("token", "token", "TOKEN"))
        .arg(count_flag("with_token", "with-token"))
}

fn auth_logout_command() -> Command {
    output_only_command("logout")
}

fn issue_list_command() -> Command {
    base_command("list")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(string_option("state", "state", "STATE"))
        .arg(string_option("search", "search", "SEARCH"))
        .arg(string_option("page", "page", "PAGE"))
        .arg(string_option("per_page", "per-page", "PER_PAGE"))
}

fn issue_view_command() -> Command {
    base_command("view")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(count_flag("comments", "comments"))
        .arg(string_option("page", "page", "PAGE"))
        .arg(string_option("per_page", "per-page", "PER_PAGE"))
        .arg(positionals_arg())
}

fn issue_comment_command() -> Command {
    base_command("comment")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(string_option("body", "body", "BODY"))
        .arg(string_option("body_file", "body-file", "PATH"))
        .arg(positionals_arg())
}

fn pr_view_command() -> Command {
    base_command("view")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(positionals_arg())
}

fn repo_view_command() -> Command {
    output_only_command("view").arg(string_option("repo", "repo", "REPO"))
}

fn repo_clone_command() -> Command {
    base_command("clone")
        .arg(json_flag())
        .arg(count_flag("https", "https"))
        .arg(count_flag("ssh", "ssh"))
        .arg(positionals_arg())
}

fn pr_comment_command() -> Command {
    base_command("comment")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(string_option("body", "body", "BODY"))
        .arg(string_option("body_file", "body-file", "PATH"))
        .arg(positionals_arg())
}

fn pr_list_command() -> Command {
    base_command("list")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(string_option("state", "state", "STATE"))
        .arg(string_option("author", "author", "AUTHOR"))
        .arg(string_option("assignee", "assignee", "ASSIGNEE"))
        .arg(string_option("base", "base", "BASE"))
        .arg(string_option("head", "head", "HEAD"))
        .arg(string_option("limit", "limit", "LIMIT"))
}

fn pr_create_command() -> Command {
    base_command("create")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(string_option("head", "head", "HEAD"))
        .arg(string_option("base", "base", "BASE"))
        .arg(string_option("title", "title", "TITLE"))
        .arg(string_option("body", "body", "BODY"))
        .arg(string_option("body_file", "body-file", "PATH"))
}

fn pr_checkout_command() -> Command {
    base_command("checkout")
        .arg(json_flag())
        .arg(string_option("repo", "repo", "REPO"))
        .arg(positionals_arg())
}

fn pr_status_command() -> Command {
    base_command("status")
        .arg(json_flag())
        .arg(string_option("state", "state", "STATE"))
        .arg(string_option("limit", "limit", "LIMIT"))
}

fn output_only_command(name: &'static str) -> Command {
    base_command(name).arg(json_flag())
}

fn base_command(name: &'static str) -> Command {
    Command::new(name).disable_version_flag(true)
}

fn json_flag() -> Arg {
    count_flag("json", "json")
}

fn count_flag(id: &'static str, long: &'static str) -> Arg {
    Arg::new(id).long(long).action(ArgAction::Count)
}

fn string_option(id: &'static str, long: &'static str, value_name: &'static str) -> Arg {
    Arg::new(id)
        .long(long)
        .action(ArgAction::Append)
        .num_args(1)
        .value_name(value_name)
        .allow_hyphen_values(true)
}

fn positionals_arg() -> Arg {
    Arg::new("positionals")
        .index(1)
        .action(ArgAction::Append)
        .num_args(0..)
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

fn parse_positive_integer_flag(flag: &str, value: &str) -> Result<u32, CommandError> {
    let parsed = value.parse::<u32>().ok().filter(|candidate| *candidate > 0);

    parsed.ok_or_else(|| {
        CommandError::usage(format!(
            "invalid value for {flag}: expected a positive integer"
        ))
    })
}
