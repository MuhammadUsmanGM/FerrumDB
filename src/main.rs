pub mod cli;
pub mod error;
pub mod metrics;
pub mod storage;

use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, EditMode, Editor};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use rustyline::{Helper, Completer, Hinter, Highlighter, Validator};
use std::borrow::Cow;

use crate::cli::{parse, print_help, Command};
use crate::metrics::Metrics;
use crate::storage::StorageEngine;

#[derive(Helper, Completer, Hinter, Highlighter, Validator)]
struct FerrumHelper {
    #[rustyline(Completer)]
    completer: FerrumCompleter,
    #[rustyline(Highlighter)]
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: rustyline::validate::MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
}

struct FerrumCompleter {
    engine: Arc<StorageEngine>,
}

impl Completer for FerrumCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let commands = vec!["SET ", "GET ", "DELETE ", "KEYS", "COUNT", "HELP", "EXIT"];
        let mut candidates = Vec::new();

        let upcase = line.to_uppercase();
        
        // Command completion
        if pos == line.len() {
            for cmd in &commands {
                if cmd.to_uppercase().starts_with(&upcase) {
                    candidates.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
        }

        // Key completion for GET/DELETE (Simplified for sync context)
        // Note: Real-time key completion is hard in sync rustyline without blocking.
        // We'll stick to command completion for now to keep it stable.

        Ok((0, candidates))
    }
}

fn print_banner() {
    let b1 = "\x1b[38;5;33m";
    let bold = "\x1b[1m";
    let reset = "\x1b[0m";

    println!("\n{bold}{b1}FerrumDB Premium REPL{reset}");
    println!("Type {bold}HELP{reset} for commands. Autocomplete: [TAB]\n");
}

const DATA_FILE: &str = "ferrum.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("ferrumdb=info")))
        .with_target(false)
        .init();

    print_banner();

    let engine = match StorageEngine::new(DATA_FILE).await {
        Ok(e) => Arc::new(e),
        Err(e) => {
            error!("Failed to initialize storage: {e}");
            std::process::exit(1);
        }
    };

    let metrics = Arc::new(Metrics::new());
    
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    let helper = FerrumHelper {
        completer: FerrumCompleter { engine: Arc::clone(&engine) },
        highlighter: MatchingBracketHighlighter::new(),
        validator: rustyline::validate::MatchingBracketValidator::new(),
        hinter: HistoryHinter {},
    };

    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(helper));

    loop {
        let p = "\x1b[38;5;45mferrumdb>\x1b[0m ";
        let line = match rl.readline(&p) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(e) => {
                error!("Input error: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let _ = rl.add_history_entry(trimmed);

        let cmd = match parse(trimmed) {
            Ok(cmd) => cmd,
            Err(e) => {
                metrics.record_error();
                println!("\x1b[31mError: {}\x1b[0m", e);
                continue;
            }
        };

        match cmd {
            Command::Set { key, value } => {
                metrics.record_set();
                match engine.set(key, value).await {
                    Ok(_) => println!("\x1b[32mOK\x1b[0m"),
                    Err(e) => {
                        metrics.record_error();
                        println!("\x1b[31mError: {}\x1b[0m", e);
                    }
                }
            }
            Command::Get { key } => {
                metrics.record_get();
                match engine.get(&key).await {
                    Some(val) => {
                        let pretty = serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string());
                        println!("{}", pretty);
                    }
                    None => println!("\x1b[33m(nil)\x1b[0m"),
                }
            }
            Command::Delete { key } => {
                metrics.record_delete();
                match engine.delete(&key).await {
                    Ok(Some(_)) => println!("\x1b[32mOK (deleted)\x1b[0m"),
                    Ok(None) => println!("\x1b[33m(nil) key not found\x1b[0m"),
                    Err(e) => {
                        metrics.record_error();
                        println!("\x1b[31mError: {}\x1b[0m", e);
                    }
                }
            }
            Command::Keys => {
                let keys = engine.keys().await;
                if keys.is_empty() {
                    println!("(empty)");
                } else {
                    for k in &keys {
                        println!("  \x1b[36m{}\x1b[0m", k);
                    }
                    println!("({} keys)", keys.len());
                }
            }
            Command::Count => {
                println!("{} entries", engine.len().await);
            }
            Command::Help => {
                print_help();
                println!("\x1b[34m{}\x1b[0m", metrics.summary());
            }
            Command::Exit => {
                info!("Shutting down. {}", metrics.summary());
                println!("Goodbye!");
                break;
            }
        }
    }
    Ok(())
}
