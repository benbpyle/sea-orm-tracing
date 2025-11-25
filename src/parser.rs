//! SQL parsing utilities for extracting operation type and table names.

use once_cell::sync::Lazy;
use regex::Regex;

/// SQL operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlOperation {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Truncate,
    Begin,
    Commit,
    Rollback,
    Set,
    Other,
}

impl SqlOperation {
    /// Returns the operation as a string suitable for span names.
    pub fn as_str(&self) -> &'static str {
        match self {
            SqlOperation::Select => "SELECT",
            SqlOperation::Insert => "INSERT",
            SqlOperation::Update => "UPDATE",
            SqlOperation::Delete => "DELETE",
            SqlOperation::Create => "CREATE",
            SqlOperation::Drop => "DROP",
            SqlOperation::Alter => "ALTER",
            SqlOperation::Truncate => "TRUNCATE",
            SqlOperation::Begin => "BEGIN",
            SqlOperation::Commit => "COMMIT",
            SqlOperation::Rollback => "ROLLBACK",
            SqlOperation::Set => "SET",
            SqlOperation::Other => "QUERY",
        }
    }
}

impl std::fmt::Display for SqlOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Regex patterns for table extraction (compiled once)
// Using raw string literals with proper escaping for regex character classes
static SELECT_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bFROM\s+[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static INSERT_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bINSERT\s+INTO\s+[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static UPDATE_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bUPDATE\s+[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static DELETE_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bDELETE\s+FROM\s+[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static CREATE_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bCREATE\s+(?:TEMP(?:ORARY)?\s+)?TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static DROP_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bDROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static ALTER_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bALTER\s+TABLE\s+[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

static TRUNCATE_TABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bTRUNCATE\s+(?:TABLE\s+)?[`"\[]?(\w+)[`"\]]?"#).unwrap()
});

/// Parse the SQL operation type from a query string.
pub fn parse_operation(sql: &str) -> SqlOperation {
    let trimmed = sql.trim_start();
    let upper_start: String = trimmed.chars().take(15).collect::<String>().to_uppercase();

    if upper_start.starts_with("SELECT") || upper_start.starts_with("WITH") {
        SqlOperation::Select
    } else if upper_start.starts_with("INSERT") {
        SqlOperation::Insert
    } else if upper_start.starts_with("UPDATE") {
        SqlOperation::Update
    } else if upper_start.starts_with("DELETE") {
        SqlOperation::Delete
    } else if upper_start.starts_with("CREATE") {
        SqlOperation::Create
    } else if upper_start.starts_with("DROP") {
        SqlOperation::Drop
    } else if upper_start.starts_with("ALTER") {
        SqlOperation::Alter
    } else if upper_start.starts_with("TRUNCATE") {
        SqlOperation::Truncate
    } else if upper_start.starts_with("BEGIN") || upper_start.starts_with("START") {
        SqlOperation::Begin
    } else if upper_start.starts_with("COMMIT") {
        SqlOperation::Commit
    } else if upper_start.starts_with("ROLLBACK") {
        SqlOperation::Rollback
    } else if upper_start.starts_with("SET") {
        SqlOperation::Set
    } else {
        SqlOperation::Other
    }
}

/// Extract the primary table name from a SQL query.
///
/// Returns `None` if the table cannot be determined.
pub fn extract_table(sql: &str) -> Option<String> {
    let operation = parse_operation(sql);

    let regex = match operation {
        SqlOperation::Select => &*SELECT_TABLE_REGEX,
        SqlOperation::Insert => &*INSERT_TABLE_REGEX,
        SqlOperation::Update => &*UPDATE_TABLE_REGEX,
        SqlOperation::Delete => &*DELETE_TABLE_REGEX,
        SqlOperation::Create => &*CREATE_TABLE_REGEX,
        SqlOperation::Drop => &*DROP_TABLE_REGEX,
        SqlOperation::Alter => &*ALTER_TABLE_REGEX,
        SqlOperation::Truncate => &*TRUNCATE_TABLE_REGEX,
        _ => return None,
    };

    regex
        .captures(sql)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_lowercase())
}

/// Parsed SQL information for span creation.
#[derive(Debug)]
pub struct ParsedSql {
    pub operation: SqlOperation,
    pub table: Option<String>,
}

impl ParsedSql {
    /// Parse a SQL statement and extract operation and table information.
    pub fn parse(sql: &str) -> Self {
        let operation = parse_operation(sql);
        let table = extract_table(sql);
        Self { operation, table }
    }

    /// Generate a span name from the parsed SQL.
    ///
    /// Format: "db.query {OPERATION} {table}" or "db.query {OPERATION}"
    pub fn span_name(&self) -> String {
        match &self.table {
            Some(table) => format!("{} {}", self.operation.as_str(), table),
            None => self.operation.as_str().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_select() {
        assert_eq!(parse_operation("SELECT * FROM users"), SqlOperation::Select);
        assert_eq!(parse_operation("select id from orders"), SqlOperation::Select);
        assert_eq!(
            parse_operation("WITH cte AS (SELECT 1) SELECT * FROM cte"),
            SqlOperation::Select
        );
    }

    #[test]
    fn test_parse_insert() {
        assert_eq!(
            parse_operation("INSERT INTO users (name) VALUES ('test')"),
            SqlOperation::Insert
        );
    }

    #[test]
    fn test_parse_update() {
        assert_eq!(
            parse_operation("UPDATE users SET name = 'test' WHERE id = 1"),
            SqlOperation::Update
        );
    }

    #[test]
    fn test_parse_delete() {
        assert_eq!(
            parse_operation("DELETE FROM users WHERE id = 1"),
            SqlOperation::Delete
        );
    }

    #[test]
    fn test_extract_table_select() {
        assert_eq!(
            extract_table("SELECT * FROM users WHERE id = 1"),
            Some("users".to_string())
        );
        assert_eq!(
            extract_table(r#"SELECT * FROM "Users" WHERE id = 1"#),
            Some("users".to_string())
        );
        assert_eq!(
            extract_table("select u.* from users u join orders o on u.id = o.user_id"),
            Some("users".to_string())
        );
    }

    #[test]
    fn test_extract_table_insert() {
        assert_eq!(
            extract_table("INSERT INTO grades (student_id, score) VALUES ($1, $2)"),
            Some("grades".to_string())
        );
    }

    #[test]
    fn test_extract_table_update() {
        assert_eq!(
            extract_table("UPDATE students SET name = $1 WHERE id = $2"),
            Some("students".to_string())
        );
    }

    #[test]
    fn test_extract_table_delete() {
        assert_eq!(
            extract_table("DELETE FROM assignments WHERE id = $1"),
            Some("assignments".to_string())
        );
    }

    #[test]
    fn test_parsed_sql_span_name() {
        let parsed = ParsedSql::parse("SELECT * FROM users WHERE id = 1");
        assert_eq!(parsed.span_name(), "SELECT users");

        let parsed = ParsedSql::parse("BEGIN");
        assert_eq!(parsed.span_name(), "BEGIN");
    }

    #[test]
    fn test_transaction_operations() {
        assert_eq!(parse_operation("BEGIN"), SqlOperation::Begin);
        assert_eq!(parse_operation("START TRANSACTION"), SqlOperation::Begin);
        assert_eq!(parse_operation("COMMIT"), SqlOperation::Commit);
        assert_eq!(parse_operation("ROLLBACK"), SqlOperation::Rollback);
    }
}
