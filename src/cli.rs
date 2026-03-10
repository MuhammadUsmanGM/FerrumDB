use crate::error::FerrumError;
use serde_json::Value;

/// Parsed CLI commands.
#[derive(Debug, PartialEq)]
pub enum Command {
    Set { key: String, value: Value },
    Get { key: String },
    Delete { key: String },
    Keys,
    Count,
    Help,
    Exit,
    CreateIndex { field: String },
    Find { field: String, value: Value },
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
            let value_str = parts.get(2).ok_or(FerrumError::MissingArgument("value"))?;
            
            // Try parsing as JSON, fallback to String
            let value = serde_json::from_str(value_str).unwrap_or(Value::String(value_str.to_string()));
            
            Ok(Command::Set {
                key: key.to_string(),
                value,
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
        "INDEX" | "CREATE_INDEX" => {
            let field = parts.get(1).ok_or(FerrumError::MissingArgument("field"))?;
            Ok(Command::CreateIndex { field: field.to_string() })
        }
        "FIND" => {
            let field = parts.get(1).ok_or(FerrumError::MissingArgument("field"))?;
            let value_str = parts.get(2).ok_or(FerrumError::MissingArgument("value"))?;
            let value = serde_json::from_str(value_str).unwrap_or(Value::String(value_str.to_string()));
            Ok(Command::Find { field: field.to_string(), value })
        }
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
  INDEX <field>       Create secondary index on a JSON field
  FIND <field> <val>  Search data using a secondary index
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
            Command::Set { key: "name".into(), value: Value::String("usman".into()) }
        );
    }

    #[test]
    fn test_parse_set_with_spaces_in_value() {
        assert_eq!(
            parse("SET greeting hello world").unwrap(),
            Command::Set { key: "greeting".into(), value: Value::String("hello world".into()) }
        );
    }

    #[test]
    fn test_parse_set_json() {
        assert_eq!(
            parse(r#"SET user {"id": 1}"#).unwrap(),
            Command::Set { 
                key: "user".into(), 
                value: serde_json::json!({"id": 1}) 
            }
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
