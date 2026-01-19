use std::fmt;

/// Money is represented as integer cents to avoid floating-point precision issues.
/// For EUR/USD, 1 unit = 100 cents, so €50.00 = 5000 cents.
pub type Cents = i64;

/// Format cents as a human-readable currency string.
/// Example: 5000 -> "50.00", -1234 -> "-12.34"
pub fn format_cents(cents: Cents) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs_cents = cents.abs();
    let units = abs_cents / 100;
    let remainder = abs_cents % 100;
    format!("{}{}.{:02}", sign, units, remainder)
}

/// Parse a decimal string into cents.
/// Example: "50.00" -> 5000, "12.5" -> 1250, "100" -> 10000
pub fn parse_cents(input: &str) -> Result<Cents, ParseCentsError> {
    let input = input.trim();
    let negative = input.starts_with('-');
    let input = input.trim_start_matches('-');

    let parts: Vec<&str> = input.split('.').collect();
    match parts.len() {
        1 => {
            // No decimal point, treat as whole units
            let units: i64 = parts[0]
                .parse()
                .map_err(|_| ParseCentsError::InvalidFormat)?;
            let cents = units * 100;
            Ok(if negative { -cents } else { cents })
        }
        2 => {
            let units: i64 = if parts[0].is_empty() {
                0
            } else {
                parts[0]
                    .parse()
                    .map_err(|_| ParseCentsError::InvalidFormat)?
            };

            // Handle decimal part - pad or truncate to 2 digits
            let decimal_str = parts[1];
            let decimal_cents: i64 = match decimal_str.len() {
                0 => 0,
                1 => {
                    // Single digit like "5" means 50 cents
                    decimal_str
                        .parse::<i64>()
                        .map_err(|_| ParseCentsError::InvalidFormat)?
                        * 10
                }
                2 => decimal_str
                    .parse()
                    .map_err(|_| ParseCentsError::InvalidFormat)?,
                _ => {
                    // More than 2 decimal places - truncate
                    decimal_str[..2]
                        .parse()
                        .map_err(|_| ParseCentsError::InvalidFormat)?
                }
            };

            let cents = units * 100 + decimal_cents;
            Ok(if negative { -cents } else { cents })
        }
        _ => Err(ParseCentsError::InvalidFormat),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseCentsError {
    InvalidFormat,
}

impl fmt::Display for ParseCentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseCentsError::InvalidFormat => write!(f, "invalid money format"),
        }
    }
}

impl std::error::Error for ParseCentsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cents() {
        assert_eq!(format_cents(5000), "50.00");
        assert_eq!(format_cents(1234), "12.34");
        assert_eq!(format_cents(100), "1.00");
        assert_eq!(format_cents(1), "0.01");
        assert_eq!(format_cents(0), "0.00");
        assert_eq!(format_cents(-5000), "-50.00");
        assert_eq!(format_cents(-1), "-0.01");
    }

    #[test]
    fn test_parse_cents() {
        assert_eq!(parse_cents("50.00"), Ok(5000));
        assert_eq!(parse_cents("50"), Ok(5000));
        assert_eq!(parse_cents("12.34"), Ok(1234));
        assert_eq!(parse_cents("12.5"), Ok(1250));
        assert_eq!(parse_cents("0.01"), Ok(1));
        assert_eq!(parse_cents(".50"), Ok(50));
        assert_eq!(parse_cents("-50.00"), Ok(-5000));
        assert_eq!(parse_cents("100.999"), Ok(10099)); // Truncates
    }

    #[test]
    fn test_parse_cents_invalid() {
        assert!(parse_cents("abc").is_err());
        assert!(parse_cents("12.34.56").is_err());
    }
}
