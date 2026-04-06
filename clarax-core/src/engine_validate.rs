// Author: Abdulwahed Mansour
//! Validation engine with Rayon parallel support.
//!
//! Validates batches of field values against their descriptors, returning
//! structured errors. For batches above PARALLEL_THRESHOLD, validation
//! runs across CPU cores via Rayon's work-stealing thread pool.

use rayon::prelude::*;
use std::collections::HashMap;

use crate::error::FieldValidationError;
use crate::types::{FieldDescriptor, FieldType, FieldValue};

/// Minimum batch size before Rayon parallelization kicks in.
///
/// Without Django's `getattr` overhead, per-field validation is faster,
/// so the crossover point where Rayon's dispatch cost is recovered is higher.
pub const PARALLEL_THRESHOLD: usize = 128;

/// Outcome of validating a batch of field values.
#[derive(Debug)]
pub struct ValidationReport {
    /// Number of fields that passed all validation checks.
    pub valid_count: usize,
    /// Number of individual error instances (one field can produce multiple).
    pub error_count: usize,
    /// Per-field error details, ordered by input position.
    pub field_errors: Vec<FieldValidationError>,
}

impl ValidationReport {
    /// Returns `true` if every field in the batch passed validation.
    pub fn is_valid(&self) -> bool {
        self.field_errors.is_empty()
    }
}

/// Validates a flat buffer of field values against a repeating descriptor pattern.
///
/// `values` is laid out as `[rec0_field0, rec0_field1, ..., rec1_field0, ...]`.
/// The descriptor slice is cycled to match. This avoids cloning descriptors
/// for every record and enables chunk-based parallelism at the record level.
pub fn validate_batch_chunked(
    descriptors: &[FieldDescriptor],
    values: &[FieldValue],
) -> ValidationReport {
    let fields_per_record = descriptors.len();
    if fields_per_record == 0 || values.is_empty() {
        return ValidationReport {
            valid_count: 0,
            error_count: 0,
            field_errors: Vec::new(),
        };
    }

    let num_records = values.len() / fields_per_record;

    let chunk_errors: Vec<Vec<FieldValidationError>> = if num_records >= PARALLEL_THRESHOLD {
        values
            .par_chunks(fields_per_record)
            .map(|record_values| {
                let mut errors = Vec::new();
                for (desc, val) in descriptors.iter().zip(record_values.iter()) {
                    errors.append(&mut validate_single(desc, val));
                }
                errors
            })
            .collect()
    } else {
        values
            .chunks(fields_per_record)
            .map(|record_values| {
                let mut errors = Vec::new();
                for (desc, val) in descriptors.iter().zip(record_values.iter()) {
                    errors.append(&mut validate_single(desc, val));
                }
                errors
            })
            .collect()
    };

    let all_errors: Vec<FieldValidationError> =
        chunk_errors.into_iter().flatten().collect();
    let entries_with_errors = all_errors
        .iter()
        .map(|e| &e.field_name)
        .collect::<std::collections::HashSet<_>>()
        .len();

    ValidationReport {
        valid_count: values.len().saturating_sub(entries_with_errors),
        error_count: all_errors.len(),
        field_errors: all_errors,
    }
}

/// Validates a batch of (descriptor, value) pairs.
///
/// For batches above `PARALLEL_THRESHOLD`, validation runs in parallel.
/// Results are always returned in input order regardless of parallelism.
pub fn validate_batch(entries: &[(FieldDescriptor, FieldValue)]) -> ValidationReport {
    let mut indexed_errors: Vec<(usize, Vec<FieldValidationError>)> =
        if entries.len() >= PARALLEL_THRESHOLD {
            entries
                .par_iter()
                .enumerate()
                .map(|(i, (desc, val))| (i, validate_single(desc, val)))
                .collect()
        } else {
            entries
                .iter()
                .enumerate()
                .map(|(i, (desc, val))| (i, validate_single(desc, val)))
                .collect()
        };

    indexed_errors.sort_by_key(|(i, _)| *i);

    let errors: Vec<FieldValidationError> = indexed_errors
        .into_iter()
        .flat_map(|(_, errs)| errs)
        .collect();

    let entries_with_errors = errors
        .iter()
        .map(|e| &e.field_name)
        .collect::<std::collections::HashSet<_>>()
        .len();

    ValidationReport {
        valid_count: entries.len().saturating_sub(entries_with_errors),
        error_count: errors.len(),
        field_errors: errors,
    }
}

/// Validates a single field value against its descriptor.
pub fn validate_single(
    descriptor: &FieldDescriptor,
    value: &FieldValue,
) -> Vec<FieldValidationError> {
    let mut errors = Vec::new();

    if matches!(value, FieldValue::Null) {
        if !descriptor.nullable && !descriptor.has_default {
            errors.push(FieldValidationError {
                field_name: descriptor.name.clone(),
                message: "This field is required.".into(),
                code: "required".into(),
                params: HashMap::new(),
            });
        }
        return errors;
    }

    match (&descriptor.field_type, value) {
        // String fields
        (FieldType::Str { max_length, min_length }, FieldValue::Text(s)) => {
            let char_count = s.chars().count();
            if let Some(max) = max_length {
                if char_count > *max {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!(
                            "Ensure this value has at most {max} characters (it has {char_count})."
                        ),
                        code: "max_length".into(),
                        params: HashMap::from([
                            ("max_length".into(), max.to_string()),
                            ("length".into(), char_count.to_string()),
                        ]),
                    });
                }
            }
            if let Some(min) = min_length {
                if char_count < *min {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!(
                            "Ensure this value has at least {min} characters (it has {char_count})."
                        ),
                        code: "min_length".into(),
                        params: HashMap::from([
                            ("min_length".into(), min.to_string()),
                            ("length".into(), char_count.to_string()),
                        ]),
                    });
                }
            }
        }

        // Integer fields with bounds
        (FieldType::Int { min_value, max_value }, FieldValue::Integer(n)) => {
            if let Some(min) = min_value {
                if n < min {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!("Ensure this value is greater than or equal to {min}."),
                        code: "min_value".into(),
                        params: HashMap::from([("min_value".into(), min.to_string())]),
                    });
                }
            }
            if let Some(max) = max_value {
                if n > max {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!("Ensure this value is less than or equal to {max}."),
                        code: "max_value".into(),
                        params: HashMap::from([("max_value".into(), max.to_string())]),
                    });
                }
            }
        }

        // Float fields with bounds
        (FieldType::Float { min_value, max_value }, FieldValue::Float(f)) => {
            if let Some(min) = min_value {
                if f < min {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!("Ensure this value is greater than or equal to {min}."),
                        code: "min_value".into(),
                        params: HashMap::from([("min_value".into(), min.to_string())]),
                    });
                }
            }
            if let Some(max) = max_value {
                if f > max {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!("Ensure this value is less than or equal to {max}."),
                        code: "max_value".into(),
                        params: HashMap::from([("max_value".into(), max.to_string())]),
                    });
                }
            }
        }

        // Decimal fields
        (FieldType::Decimal { max_digits, decimal_places }, FieldValue::Decimal(d)) => {
            let mantissa_abs = d.mantissa().unsigned_abs();
            let total_digits = if mantissa_abs == 0 { 1u32 } else { mantissa_abs.ilog10() + 1 };
            let scale = d.scale();

            if let Some(max_d) = max_digits {
                if total_digits > *max_d {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!(
                            "Ensure that there are no more than {max_d} digits in total."
                        ),
                        code: "max_digits".into(),
                        params: HashMap::from([("max_digits".into(), max_d.to_string())]),
                    });
                }
            }
            if let Some(dp) = decimal_places {
                if scale > *dp {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!(
                            "Ensure that there are no more than {dp} decimal places."
                        ),
                        code: "max_decimal_places".into(),
                        params: HashMap::from([("decimal_places".into(), dp.to_string())]),
                    });
                }
            }
        }

        // Simple type matches — no additional constraints
        (FieldType::Bool, FieldValue::Boolean(_)) => {}
        (FieldType::Date, FieldValue::Date(_)) => {}
        (FieldType::Time, FieldValue::Time(_)) => {}
        (FieldType::DateTime, FieldValue::DateTime(_)) => {}
        (FieldType::Uuid, FieldValue::Uuid(_)) => {}
        (FieldType::List, FieldValue::Json(serde_json::Value::Array(_))) => {}
        (FieldType::Dict, FieldValue::Json(serde_json::Value::Object(_))) => {}
        (FieldType::List, FieldValue::Json(_)) => {
            errors.push(FieldValidationError {
                field_name: descriptor.name.clone(),
                message: "Expected a list.".into(),
                code: "invalid".into(),
                params: HashMap::new(),
            });
        }
        (FieldType::Dict, FieldValue::Json(_)) => {
            errors.push(FieldValidationError {
                field_name: descriptor.name.clone(),
                message: "Expected a dict.".into(),
                code: "invalid".into(),
                params: HashMap::new(),
            });
        }

        // Bytes with length constraint
        (FieldType::Bytes { max_length }, FieldValue::Binary(bytes)) => {
            if let Some(max) = max_length {
                if bytes.len() > *max {
                    errors.push(FieldValidationError {
                        field_name: descriptor.name.clone(),
                        message: format!(
                            "Ensure this value has at most {max} bytes (it has {}).",
                            bytes.len()
                        ),
                        code: "max_length".into(),
                        params: HashMap::from([("max_length".into(), max.to_string())]),
                    });
                }
            }
        }

        // Type mismatch
        (field_type, value) => {
            errors.push(FieldValidationError {
                field_name: descriptor.name.clone(),
                message: format!(
                    "Invalid type: expected {}, got {}.",
                    field_type.type_name(),
                    value.type_name()
                ),
                code: "invalid".into(),
                params: HashMap::new(),
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn str_field(name: &str, max_length: Option<usize>) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            field_type: FieldType::Str { max_length, min_length: None },
            nullable: false,
            has_default: false,
        }
    }

    fn int_field(name: &str, min: Option<i64>, max: Option<i64>) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            field_type: FieldType::Int { min_value: min, max_value: max },
            nullable: false,
            has_default: false,
        }
    }

    fn decimal_field(name: &str, max_digits: u32, decimal_places: u32) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            field_type: FieldType::Decimal {
                max_digits: Some(max_digits),
                decimal_places: Some(decimal_places),
            },
            nullable: false,
            has_default: false,
        }
    }

    #[test]
    fn valid_str_passes() {
        let entries = vec![(str_field("name", Some(100)), FieldValue::Text("Alice".into()))];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn str_exceeding_max_length_fails() {
        let entries = vec![(str_field("name", Some(5)), FieldValue::Text("Abdulwahed".into()))];
        let report = validate_batch(&entries);
        assert!(!report.is_valid());
        assert_eq!(report.field_errors[0].code, "max_length");
    }

    #[test]
    fn null_on_required_field_fails() {
        let entries = vec![(str_field("name", None), FieldValue::Null)];
        let report = validate_batch(&entries);
        assert_eq!(report.field_errors[0].code, "required");
    }

    #[test]
    fn null_on_nullable_field_passes() {
        let mut desc = str_field("bio", None);
        desc.nullable = true;
        let entries = vec![(desc, FieldValue::Null)];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn int_below_min_fails() {
        let entries = vec![(int_field("age", Some(0), Some(150)), FieldValue::Integer(-1))];
        let report = validate_batch(&entries);
        assert_eq!(report.field_errors[0].code, "min_value");
    }

    #[test]
    fn int_above_max_fails() {
        let entries = vec![(int_field("age", Some(0), Some(150)), FieldValue::Integer(200))];
        let report = validate_batch(&entries);
        assert_eq!(report.field_errors[0].code, "max_value");
    }

    #[test]
    fn int_within_bounds_passes() {
        let entries = vec![(int_field("age", Some(0), Some(150)), FieldValue::Integer(30))];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn decimal_exceeding_max_digits_fails() {
        let entries = vec![(
            decimal_field("price", 5, 2),
            FieldValue::Decimal(Decimal::new(1_234_567, 2)),
        )];
        let report = validate_batch(&entries);
        assert_eq!(report.field_errors[0].code, "max_digits");
    }

    #[test]
    fn decimal_within_limits_passes() {
        let entries = vec![(
            decimal_field("price", 10, 2),
            FieldValue::Decimal(Decimal::new(9999, 2)),
        )];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn type_mismatch_produces_error() {
        let entries = vec![(
            int_field("age", None, None),
            FieldValue::Text("not a number".into()),
        )];
        let report = validate_batch(&entries);
        assert_eq!(report.field_errors[0].code, "invalid");
    }

    #[test]
    fn large_batch_parallel_preserves_order() {
        let entries: Vec<_> = (0..200)
            .map(|i| {
                if i % 2 == 0 {
                    (str_field(&format!("f_{i:04}"), Some(100)), FieldValue::Text("ok".into()))
                } else {
                    (str_field(&format!("f_{i:04}"), Some(2)), FieldValue::Text("too long".into()))
                }
            })
            .collect();
        let report = validate_batch(&entries);
        for window in report.field_errors.windows(2) {
            assert!(window[0].field_name < window[1].field_name);
        }
    }

    #[test]
    fn multibyte_str_counts_characters_not_bytes() {
        let arabic = "\u{0639}\u{0628}\u{062F}\u{0627}\u{0644}\u{0648}\u{0627}\u{062D}\u{062F} \u{0645}";
        let entries = vec![(str_field("name", Some(12)), FieldValue::Text(arabic.into()))];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn decimal_100_has_3_digits() {
        let entries = vec![(
            decimal_field("amount", 3, 0),
            FieldValue::Decimal(Decimal::new(100, 0)),
        )];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn decimal_zero_has_1_digit() {
        let entries = vec![(
            decimal_field("amount", 1, 0),
            FieldValue::Decimal(Decimal::new(0, 0)),
        )];
        assert!(validate_batch(&entries).is_valid());
    }

    #[test]
    fn empty_batch_returns_zero() {
        let entries: Vec<(FieldDescriptor, FieldValue)> = vec![];
        let report = validate_batch(&entries);
        assert!(report.is_valid());
        assert_eq!(report.valid_count, 0);
    }
}
