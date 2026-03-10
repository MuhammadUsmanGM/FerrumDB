use crate::error::FerrumError;

/// Parsed CLI commands.
#[derive(Debug, PartialEq)]
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Delete { key: String },
    Keys,
    Count,
    Help,
    Exit,
}

/// Parse a raw input line into a Command.
pub fn parse(input: &str) -> Result<Command, FerrumError> {
    let parts: Vec<&str> = input.trim().splitn(3, char::is_whitespace).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(FerrumError::InvalidCommand(String::new()));
    }

    match parts[0].to_uppercase().as_str() {
        "SET" => {
            let key = parts.get(1).ok_or(FerrumError::MissingArgument("key"))?;
            let value = parts.get(2).ok_or(FerrumError::MissingArgument("value"))?;
            Ok(Command::Set {
                key: key.to_string(),
                value: value.to_string(),
            })
        }
        "GET" => {
            let key = parts.get(1).ok_or(FerrumError::MissingArgument("key"))?;
            Ok(Command::Get { key: key.to_string() })
        }
        "DELETE" | "DEL" => {
            let key = parts.get(1).ok_or(FerrumError::MissingArgument("key"))?;
            Ok(Command::Delete { key: key.to_string() })
        }
        "KEYS" => Ok(Command::Keys),
        "COUNT" => Ok(Command::Count),
        "HELP" => Ok(Command::Help),
        "EXIT" | "QUIT" => Ok(Command::Exit),
        other => Err(FerrumError::InvalidCommand(other.to_string())),
    }
}

pub fn print_help() {
    println!(
        r#"
FerrumDB Commands:
  SET <key> <value>   Store a key-value pair
  GET <key>           Retrieve value by key
  DELETE <key>        Remove a key-value pair
  KEYS                List all keys
  COUNT               Show number of stored entries
  HELP                Show this help message
  EXIT                Quit FerrumDB
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_set() {
        assert_eq!(
            parse("SET name usman").unwrap(),
            Command::Set { key: "name".into(), value: "usman".into() }
        );
    }

    #[test]
    fn test_parse_set_with_spaces_in_value() {
        assert_eq!(
            parse("SET greeting hello world").unwrap(),
            Command::Set { key: "greeting".into(), value: "hello world".into() }
        );
    }

    #[test]
    fn test_parse_get() {
        assert_eq!(parse("get name").unwrap(), Command::Get { key: "name".into() });
    }

    #[test]
    fn test_parse_delete() {
        assert_eq!(parse("DEL name").unwrap(), Command::Delete { key: "name".into() });
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse("INVALID").is_err());
    }
}
