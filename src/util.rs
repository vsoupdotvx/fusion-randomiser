use std::{error::Error, fmt::{Debug, Display}, hash::{DefaultHasher, Hash, Hasher}};

enum ErrorSeverity {
    Critical,
    Inconvenience,
}

pub struct CommonError {
    severity: ErrorSeverity,
    string:   String,
}

impl CommonError {
    pub fn critical(string: &str) -> Self {
        Self {
            severity: ErrorSeverity::Critical,
            string:   string.to_owned(),
        }
    }
    pub fn inconvenience(string: &str) -> Self {
        Self {
            severity: ErrorSeverity::Inconvenience,
            string:   string.to_owned(),
        }
    }
}

impl Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity_string = match self.severity {
            ErrorSeverity::Critical      => "CRITICAL",
            ErrorSeverity::Inconvenience => "Inconvenience",
        };
        f.write_str(&severity_string)?;
        f.write_str(": ")?;
        f.write_str(&self.string)?;
        Ok(())
    }
}

impl Debug for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity_string = match self.severity {
            ErrorSeverity::Critical      => "CRITICAL",
            ErrorSeverity::Inconvenience => "Inconvenience",
        };
        f.write_str(&severity_string)?;
        f.write_str(": ")?;
        f.write_str(&self.string)?;
        Ok(())
    }
}

impl Error for CommonError {
}

pub fn hash_str(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
