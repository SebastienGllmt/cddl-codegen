use cbor_event::{self, de::Deserializer};
use std::io::{BufRead, Seek};

#[derive(Debug)]
pub enum Key {
    Str(String),
    Uint(u64),
    Float(f64),
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Str(x) => write!(f, "\"{}\"", x),
            Key::Uint(x) => write!(f, "{}", x),
            Key::Float(x) => write!(f, "{}", x),
        }
    }
}

#[derive(Debug)]
pub enum DeserializeFailure {
    BreakInDefiniteLen,
    CBOR(cbor_event::Error),
    DefiniteLenMismatch(u64, Option<u64>),
    DuplicateKey(Key),
    EndingBreakMissing,
    ExpectedNull,
    FixedValueMismatch{
        found: Key,
        expected: Key,
    },
    /// Invalid internal structure imposed on top of the CBOR format
    InvalidStructure(Box<dyn std::error::Error>),
    MandatoryFieldMissing(Key),
    NoVariantMatched,
    NoVariantMatchedWithCauses(Vec<DeserializeError>),
    RangeCheck{
        found: usize,
        min: Option<isize>,
        max: Option<isize>,
    },
    TagMismatch{
        found: u64,
        expected: u64,
    },
    UnknownKey(Key),
    UnexpectedKeyType(cbor_event::Type),
}

// we might want to add more info like which field,
#[derive(Debug)]
pub struct DeserializeError {
    location: Option<String>,
    failure: DeserializeFailure,
}

impl DeserializeError {
    pub fn new<T: Into<String>>(location: T, failure: DeserializeFailure) -> Self {
        Self {
            location: Some(location.into()),
            failure,
        }
    }

    pub fn annotate<T: Into<String>>(self, location: T) -> Self {
        match self.location {
            Some(loc) => Self::new(format!("{}.{}", location.into(), loc), self.failure),
            None => Self::new(location, self.failure),
        }
    }

    fn fmt_indent(&self, f: &mut std::fmt::Formatter<'_>, indent: u32) -> std::fmt::Result {
        use std::fmt::Display;
        for _ in 0..indent {
            write!(f, "\t")?;
        }
        match &self.location {
            Some(loc) => write!(f, "Deserialization failed in {} because: ", loc),
            None => write!(f, "Deserialization: "),
        }?;
        match &self.failure {
            DeserializeFailure::BreakInDefiniteLen => write!(f, "Encountered CBOR Break while reading definite length sequence"),
            DeserializeFailure::CBOR(e) => e.fmt(f),
            DeserializeFailure::DefiniteLenMismatch(found, expected) => {
                write!(f, "Definite length mismatch: found {}", found)?;
                if let Some(expected_elems) = expected {
                    write!(f, ", expected: {}", expected_elems)?;
                }
                Ok(())
            },
            DeserializeFailure::DuplicateKey(key) => write!(f, "Duplicate key: {}", key),
            DeserializeFailure::EndingBreakMissing => write!(f, "Missing ending CBOR Break"),
            DeserializeFailure::ExpectedNull => write!(f, "Expected null, found other type"),
            DeserializeFailure::FixedValueMismatch{ found, expected } => write!(f, "Expected fixed value {} found {}", expected, found),
            DeserializeFailure::InvalidStructure(e) => {
                write!(f, "Invalid internal structure: {}", e)
            }
            DeserializeFailure::MandatoryFieldMissing(key) => write!(f, "Mandatory field {} not found", key),
            DeserializeFailure::NoVariantMatched => write!(f, "No variant matched"),
            DeserializeFailure::NoVariantMatchedWithCauses(errs) => {
                write!(f, "No variant matched. Failures:\n")?;
                for e in errs {
                    e.fmt_indent(f, indent + 1)?;
                    write!(f, "\n")?;
                }
                Ok(())
            },
            DeserializeFailure::RangeCheck{ found, min, max } => match (min, max) {
                (Some(min), Some(max)) => write!(f, "{} not in range {} - {}", found, min, max),
                (Some(min), None) => write!(f, "{} not at least {}", found, min),
                (None, Some(max)) => write!(f, "{} not at most {}", found, max),
                (None, None) => write!(f, "invalid range (no min nor max specified)"),
            },
            DeserializeFailure::TagMismatch{ found, expected } => write!(f, "Expected tag {}, found {}", expected, found),
            DeserializeFailure::UnknownKey(key) => write!(f, "Found unexpected key {}", key),
            DeserializeFailure::UnexpectedKeyType(ty) => write!(f, "Found unexpected key of CBOR type {:?}", ty),
        }
    }
}

impl std::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_indent(f, 0)
    }
}

impl From<DeserializeFailure> for DeserializeError {
    fn from(failure: DeserializeFailure) -> DeserializeError {
        DeserializeError {
            location: None,
            failure,
        }
    }
}

impl From<cbor_event::Error> for DeserializeError {
    fn from(err: cbor_event::Error) -> DeserializeError {
        DeserializeError {
            location: None,
            failure: DeserializeFailure::CBOR(err),
        }
    }
}
