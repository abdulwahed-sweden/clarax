// Author: Abdulwahed Mansour
//! # clarax-core
//!
//! Rust-accelerated serialization and validation for Python.
//! Framework-agnostic — works with any Python project.
//!
//! This crate provides the core engine that clarax-django delegates to.
//! It can also be used standalone with Flask, FastAPI, scripts, or any
//! Python code that processes structured data.

pub mod engine_serialize;
pub mod engine_validate;
pub mod error;
pub mod types;

// Re-export public types for downstream crates (clarax-django).
pub use engine_serialize::{serialize_fields, serialize_rows, SerializedRecord};
pub use engine_validate::{validate_batch, validate_batch_chunked, validate_single, ValidationReport, PARALLEL_THRESHOLD};
pub use error::{CoreError, FieldValidationError};
pub use types::{FieldDescriptor, FieldType, FieldValue};

use std::collections::HashMap;

use clarax::prelude::*;
use clarax::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rayon::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

// ─── Schema: compiled field descriptor cache ──────────────────────────────────

/// A compiled schema that caches field descriptors for repeated use.
///
/// Built once from a dict of field definitions, then reused on every
/// serialize/validate call with zero per-call parsing overhead.
///
/// Python usage:
/// ```python
/// from clarax_core import Schema, Field
/// schema = Schema({"name": Field(str, max_length=100), "age": Field(int)})
/// ```
#[pyclass]
#[derive(Clone)]
pub struct Schema {
    descriptors: Vec<FieldDescriptor>,
    field_names: Vec<String>,
    /// Indices of fields that need conversion during serialization
    /// (Decimal→str, UUID→str, DateTime/Date/Time→isoformat, Bytes→base64).
    /// Fields not listed here are passthrough (str, int, float, bool, list, dict).
    convert_indices: Vec<usize>,
}

#[pymethods]
impl Schema {
    /// Compiles a schema from a dict of field definitions.
    ///
    /// Each key is a field name, each value is a `Field` instance
    /// describing the type and constraints.
    #[new]
    fn new(fields: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut descriptors = Vec::with_capacity(fields.len());
        let mut field_names = Vec::with_capacity(fields.len());

        for (key, value) in fields.iter() {
            let name: String = key.extract()?;
            let field: Field = value.extract()?;

            field_names.push(name.clone());
            descriptors.push(FieldDescriptor {
                name,
                field_type: field.field_type,
                nullable: field.nullable,
                has_default: field.has_default,
            });
        }

        // Pre-classify which fields need conversion during serialization.
        let convert_indices: Vec<usize> = descriptors
            .iter()
            .enumerate()
            .filter(|(_, d)| matches!(
                d.field_type,
                FieldType::Decimal { .. }
                    | FieldType::Uuid
                    | FieldType::DateTime
                    | FieldType::Date
                    | FieldType::Time
                    | FieldType::Bytes { .. }
            ))
            .map(|(i, _)| i)
            .collect();

        Ok(Schema {
            descriptors,
            field_names,
            convert_indices,
        })
    }

    /// Returns the list of field names in declaration order.
    #[getter]
    fn field_names_list(&self) -> Vec<String> {
        self.field_names.clone()
    }

    /// Returns the number of fields in the schema.
    fn __len__(&self) -> usize {
        self.descriptors.len()
    }

    fn __repr__(&self) -> String {
        format!("Schema({} fields)", self.descriptors.len())
    }
}

// ─── Field: single field definition ───────────────────────────────────────────

/// Defines a single field's type and constraints.
///
/// Python usage:
/// ```python
/// from clarax_core import Field
/// from decimal import Decimal
/// from datetime import datetime
///
/// Field(str, max_length=100)
/// Field(int, min_value=0, max_value=150)
/// Field(Decimal, max_digits=10, decimal_places=2)
/// Field(datetime)
/// Field(str, nullable=True)
/// ```
#[pyclass]
#[derive(Clone)]
pub struct Field {
    field_type: FieldType,
    nullable: bool,
    has_default: bool,
}

#[pymethods]
impl Field {
    /// Creates a new field definition.
    ///
    /// Args:
    ///     python_type: The Python type (str, int, float, bool, Decimal, datetime, date, time, UUID, list, dict, bytes).
    ///     max_length: Maximum string length or byte length.
    ///     min_length: Minimum string length.
    ///     min_value: Minimum numeric value (int or float).
    ///     max_value: Maximum numeric value (int or float).
    ///     max_digits: Maximum total digits for Decimal.
    ///     decimal_places: Maximum decimal places for Decimal.
    ///     nullable: Whether None is allowed (default False).
    ///     default: Whether the field has a default value (default False).
    #[new]
    #[pyo3(signature = (python_type, *, max_length=None, min_length=None, min_value=None, max_value=None, max_digits=None, decimal_places=None, nullable=false, default=false))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        python_type: &Bound<'_, PyAny>,
        max_length: Option<usize>,
        min_length: Option<usize>,
        min_value: Option<&Bound<'_, PyAny>>,
        max_value: Option<&Bound<'_, PyAny>>,
        max_digits: Option<u32>,
        decimal_places: Option<u32>,
        nullable: bool,
        default: bool,
    ) -> PyResult<Self> {
        let type_name = python_type
            .getattr("__name__")
            .and_then(|n| n.extract::<String>())
            .unwrap_or_default();

        // Validate constraints match the declared type
        validate_constraints(
            &type_name, max_length, min_length, min_value, max_value,
            max_digits, decimal_places,
        )?;

        let field_type = match type_name.as_str() {
            "str" => FieldType::Str { max_length, min_length },
            "int" => FieldType::Int {
                min_value: extract_opt_i64(min_value)?,
                max_value: extract_opt_i64(max_value)?,
            },
            "float" => FieldType::Float {
                min_value: extract_opt_f64(min_value)?,
                max_value: extract_opt_f64(max_value)?,
            },
            "bool" => FieldType::Bool,
            "Decimal" => FieldType::Decimal { max_digits, decimal_places },
            "datetime" => FieldType::DateTime,
            "date" => FieldType::Date,
            "time" => FieldType::Time,
            "UUID" => FieldType::Uuid,
            "list" => FieldType::List,
            "dict" => FieldType::Dict,
            "bytes" => FieldType::Bytes { max_length },
            other => {
                return Err(CoreError::SchemaError {
                    message: format!("unsupported type: '{other}'. Supported: str, int, float, bool, Decimal, datetime, date, time, UUID, list, dict, bytes"),
                }
                .into());
            }
        };

        Ok(Field {
            field_type,
            nullable,
            has_default: default,
        })
    }

    fn __repr__(&self) -> String {
        format!("Field({}, nullable={})", self.field_type.type_name(), self.nullable)
    }
}

/// Validates that constraints are appropriate for the declared type.
/// Raises SchemaError immediately if a constraint doesn't apply.
#[allow(clippy::too_many_arguments)]
fn validate_constraints(
    type_name: &str,
    max_length: Option<usize>,
    min_length: Option<usize>,
    min_value: Option<&Bound<'_, PyAny>>,
    max_value: Option<&Bound<'_, PyAny>>,
    max_digits: Option<u32>,
    decimal_places: Option<u32>,
) -> PyResult<()> {
    let has_length = max_length.is_some() || min_length.is_some();
    let has_value = min_value.is_some() || max_value.is_some();
    let has_decimal = max_digits.is_some() || decimal_places.is_some();

    match type_name {
        "str" => {
            if has_value {
                return Err(CoreError::SchemaError {
                    message: "Field(str) does not support min_value/max_value. Use max_length/min_length instead.".into(),
                }.into());
            }
            if has_decimal {
                return Err(CoreError::SchemaError {
                    message: "Field(str) does not support max_digits/decimal_places.".into(),
                }.into());
            }
        }
        "int" | "float" => {
            if has_length {
                return Err(CoreError::SchemaError {
                    message: format!("Field({type_name}) does not support max_length/min_length. Use min_value/max_value instead."),
                }.into());
            }
            if has_decimal {
                return Err(CoreError::SchemaError {
                    message: format!("Field({type_name}) does not support max_digits/decimal_places."),
                }.into());
            }
        }
        "Decimal" => {
            if has_length {
                return Err(CoreError::SchemaError {
                    message: "Field(Decimal) does not support max_length/min_length. Use max_digits/decimal_places instead.".into(),
                }.into());
            }
            if has_value {
                return Err(CoreError::SchemaError {
                    message: "Field(Decimal) does not support min_value/max_value. Use max_digits/decimal_places instead.".into(),
                }.into());
            }
        }
        "bool" | "datetime" | "date" | "time" | "UUID" | "list" | "dict" => {
            if has_length || has_value || has_decimal {
                return Err(CoreError::SchemaError {
                    message: format!("Field({type_name}) does not support constraints. Only nullable and default are valid."),
                }.into());
            }
        }
        "bytes" => {
            if has_value || has_decimal {
                return Err(CoreError::SchemaError {
                    message: "Field(bytes) only supports max_length. Not min_value/max_value or max_digits.".into(),
                }.into());
            }
        }
        _ => {}
    }
    Ok(())
}

fn extract_opt_i64(val: Option<&Bound<'_, PyAny>>) -> PyResult<Option<i64>> {
    match val {
        Some(v) => Ok(Some(v.extract::<i64>()?)),
        None => Ok(None),
    }
}

fn extract_opt_f64(val: Option<&Bound<'_, PyAny>>) -> PyResult<Option<f64>> {
    match val {
        Some(v) => Ok(Some(v.extract::<f64>()?)),
        None => Ok(None),
    }
}

// ─── Python-exposed functions ─────────────────────────────────────────────────

/// Serializes a Python dict or object using a precompiled schema.
///
/// Accepts either a dict (keys are field names) or any object with
/// attributes matching the schema field names.
///
/// Returns a dict of serialized field values (JSON-compatible).
#[pyfunction]
fn serialize<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
    schema: &Schema,
) -> PyResult<Bound<'py, PyDict>> {
    let values = extract_values(py, data, &schema.descriptors)?;
    let record = serialize_fields(&schema.descriptors, &values)
        .map_err(|e| -> clarax::PyErr { e.into() })?;
    record_to_pydict(py, &record)
}

/// Serializes a list of dicts or objects using a precompiled schema.
///
/// Returns a list of dicts. For dict inputs, uses `PyDict_Copy` to shallow-copy
/// the entire input dict in one C call, then only overwrites fields that need
/// conversion (Decimal→str, UUID→str, DateTime→isoformat, Bytes→base64).
/// Passthrough fields (str, int, float, bool, list, dict) are never touched.
#[pyfunction]
fn serialize_many<'py>(
    py: Python<'py>,
    data_list: &Bound<'py, PyList>,
    schema: &Schema,
) -> PyResult<Bound<'py, PyList>> {
    let result = PyList::empty(py);

    // Pre-intern field name strings for convert fields only
    let convert_keys: Vec<(&FieldDescriptor, Bound<'_, PyString>)> = schema
        .convert_indices
        .iter()
        .map(|&i| (&schema.descriptors[i], PyString::intern(py, &schema.field_names[i])))
        .collect();

    let has_converts = !convert_keys.is_empty();

    for item in data_list.iter() {
        if let Ok(input_dict) = item.cast::<PyDict>() {
            // Fast path: shallow-copy the dict (one C call copies all entries),
            // then overwrite only the fields that need type conversion.
            let output = input_dict.copy()?;

            if has_converts {
                for (desc, key) in &convert_keys {
                    let py_val = match output.get_item(key)? {
                        Some(v) if !v.is_none() => v,
                        _ => {
                            if desc.nullable || desc.has_default {
                                continue;
                            }
                            return Err(CoreError::NullField {
                                field: desc.name.clone(),
                            }
                            .into());
                        }
                    };
                    match &desc.field_type {
                        FieldType::Decimal { .. } | FieldType::Uuid => {
                            output.set_item(key, py_val.str()?)?;
                        }
                        FieldType::DateTime | FieldType::Date | FieldType::Time => {
                            output.set_item(key, py_val.call_method0("isoformat")?)?;
                        }
                        FieldType::Bytes { .. } => {
                            let base64_mod = py.import("base64")?;
                            let encoded = base64_mod.call_method1("b64encode", (&py_val,))?;
                            output.set_item(
                                key,
                                encoded.call_method1("decode", ("ascii",))?,
                            )?;
                        }
                        _ => {}
                    }
                }
            }
            result.append(output)?;
        } else {
            // Fallback for non-dict objects (attribute access path)
            let dict = serialize(py, &item, schema)?;
            result.append(dict)?;
        }
    }
    Ok(result)
}

/// Validates a Python dict or object against a schema.
///
/// Returns a dict with `is_valid`, `valid_count`, `error_count`, and `errors`.
#[pyfunction]
fn validate<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
    schema: &Schema,
) -> PyResult<Bound<'py, PyDict>> {
    let mut batch = Vec::with_capacity(schema.descriptors.len());
    for desc in &schema.descriptors {
        let fv = extract_single_value(py, data, desc)?;
        batch.push((desc.clone(), fv));
    }
    let report = validate_batch(&batch);
    report_to_pydict(py, &report)
}

/// Validates a list of dicts or objects against a schema.
///
/// Returns a combined validation report. For dict inputs, validates inline
/// without creating intermediate FieldValue representations — avoids heap
/// allocations for str fields (uses Python `len()` directly) and skips
/// extraction entirely for bool fields (type check only).
#[pyfunction]
fn validate_many<'py>(
    py: Python<'py>,
    data_list: &Bound<'py, PyList>,
    schema: &Schema,
) -> PyResult<Bound<'py, PyDict>> {
    let num_fields = schema.descriptors.len();
    let num_records = data_list.len();
    let total_fields = num_records * num_fields;

    // Pre-intern field name strings for fast dict lookup
    let py_names: Vec<Bound<'_, PyString>> = schema
        .field_names
        .iter()
        .map(|n| PyString::intern(py, n))
        .collect();

    let mut all_errors: Vec<FieldValidationError> = Vec::new();

    for item in data_list.iter() {
        if let Ok(input_dict) = item.cast::<PyDict>() {
            for (i, desc) in schema.descriptors.iter().enumerate() {
                let py_val = input_dict.get_item(&py_names[i])?;
                let py_val = match py_val {
                    Some(v) if !v.is_none() => v,
                    _ => {
                        if !desc.nullable && !desc.has_default {
                            all_errors.push(FieldValidationError {
                                field_name: desc.name.clone(),
                                message: "This field is required.".into(),
                                code: "required".into(),
                                params: HashMap::new(),
                            });
                        }
                        continue;
                    }
                };
                validate_py_value_inline(&py_val, desc, &mut all_errors);
            }
        } else {
            // Fallback: extract to FieldValue for non-dict objects
            for desc in &schema.descriptors {
                let fv = extract_single_value(py, &item, desc)?;
                all_errors.extend(validate_single(desc, &fv));
            }
        }
    }

    let entries_with_errors = all_errors
        .iter()
        .map(|e| &e.field_name)
        .collect::<std::collections::HashSet<_>>()
        .len();

    let report = ValidationReport {
        valid_count: total_fields.saturating_sub(entries_with_errors),
        error_count: all_errors.len(),
        field_errors: all_errors,
    };
    report_to_pydict(py, &report)
}

/// Validates a Python value directly against a field descriptor without
/// creating an intermediate `FieldValue`.
///
/// For str fields: calls Python `len()` (O(1) in CPython) instead of
/// extracting a full Rust String. For bool: type-checks without extraction.
/// For int/float: extracts the primitive (cheap, no heap allocation).
fn validate_py_value_inline(
    val: &Bound<'_, PyAny>,
    desc: &FieldDescriptor,
    errors: &mut Vec<FieldValidationError>,
) {
    match &desc.field_type {
        FieldType::Str { max_length, min_length } => {
            // Python str.__len__() returns character (code point) count — O(1) in CPython.
            // This avoids extracting a full Rust String just to count characters.
            let char_count = match val.len() {
                Ok(n) => n,
                Err(_) => {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected str.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                    return;
                }
            };
            if let Some(max) = max_length {
                if char_count > *max {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
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
                        field_name: desc.name.clone(),
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
        FieldType::Int { min_value, max_value } => {
            let n: i64 = match val.extract() {
                Ok(v) => v,
                Err(_) => {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected int.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                    return;
                }
            };
            if let Some(min) = min_value {
                if n < *min {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: format!("Ensure this value is greater than or equal to {min}."),
                        code: "min_value".into(),
                        params: HashMap::from([("min_value".into(), min.to_string())]),
                    });
                }
            }
            if let Some(max) = max_value {
                if n > *max {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: format!("Ensure this value is less than or equal to {max}."),
                        code: "max_value".into(),
                        params: HashMap::from([("max_value".into(), max.to_string())]),
                    });
                }
            }
        }
        FieldType::Float { min_value, max_value } => {
            let f: f64 = match val.extract() {
                Ok(v) => v,
                Err(_) => {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected float.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                    return;
                }
            };
            if let Some(min) = min_value {
                if f < *min {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: format!("Ensure this value is greater than or equal to {min}."),
                        code: "min_value".into(),
                        params: HashMap::from([("min_value".into(), min.to_string())]),
                    });
                }
            }
            if let Some(max) = max_value {
                if f > *max {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: format!("Ensure this value is less than or equal to {max}."),
                        code: "max_value".into(),
                        params: HashMap::from([("max_value".into(), max.to_string())]),
                    });
                }
            }
        }
        FieldType::Bool => {
            // Type check only — no extraction needed
            if !val.is_instance_of::<PyBool>() {
                errors.push(FieldValidationError {
                    field_name: desc.name.clone(),
                    message: "Invalid type: expected bool.".into(),
                    code: "invalid".into(),
                    params: HashMap::new(),
                });
            }
        }
        FieldType::Decimal { max_digits, decimal_places } => {
            // Zero-copy: borrow the string from Python's internal buffer,
            // then count digits/scale directly without parsing into rust_decimal.
            let py_str = match val.str() {
                Ok(s) => s,
                Err(_) => {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected Decimal.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                    return;
                }
            };
            let s = match py_str.to_str() {
                Ok(s) => s,
                Err(_) => {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected Decimal.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                    return;
                }
            };
            let (total_digits, scale) = count_decimal_digits(s);
            if let Some(max_d) = max_digits {
                if total_digits > *max_d {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
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
                        field_name: desc.name.clone(),
                        message: format!(
                            "Ensure that there are no more than {dp} decimal places."
                        ),
                        code: "max_decimal_places".into(),
                        params: HashMap::from([("decimal_places".into(), dp.to_string())]),
                    });
                }
            }
        }
        FieldType::DateTime | FieldType::Date | FieldType::Time => {
            // Type check via isoformat() — if it has the method, it's the right type
            if val.call_method0("isoformat").is_err() {
                errors.push(FieldValidationError {
                    field_name: desc.name.clone(),
                    message: format!("Invalid type: expected {}.", desc.field_type.type_name()),
                    code: "invalid".into(),
                    params: HashMap::new(),
                });
            }
        }
        FieldType::Uuid => {
            // Just verify it can be stringified as a UUID
            if let Ok(s) = val.str().and_then(|s| s.extract::<String>()) {
                if Uuid::parse_str(&s).is_err() {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected UUID.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                }
            } else {
                errors.push(FieldValidationError {
                    field_name: desc.name.clone(),
                    message: "Invalid type: expected UUID.".into(),
                    code: "invalid".into(),
                    params: HashMap::new(),
                });
            }
        }
        FieldType::List => {
            if !val.is_instance_of::<PyList>() {
                errors.push(FieldValidationError {
                    field_name: desc.name.clone(),
                    message: "Expected a list.".into(),
                    code: "invalid".into(),
                    params: HashMap::new(),
                });
            }
        }
        FieldType::Dict => {
            if !val.is_instance_of::<PyDict>() {
                errors.push(FieldValidationError {
                    field_name: desc.name.clone(),
                    message: "Expected a dict.".into(),
                    code: "invalid".into(),
                    params: HashMap::new(),
                });
            }
        }
        FieldType::Bytes { max_length } => {
            match val.len() {
                Ok(byte_len) => {
                    if let Some(max) = max_length {
                        if byte_len > *max {
                            errors.push(FieldValidationError {
                                field_name: desc.name.clone(),
                                message: format!(
                                    "Ensure this value has at most {max} bytes (it has {byte_len})."
                                ),
                                code: "max_length".into(),
                                params: HashMap::from([("max_length".into(), max.to_string())]),
                            });
                        }
                    }
                }
                Err(_) => {
                    errors.push(FieldValidationError {
                        field_name: desc.name.clone(),
                        message: "Invalid type: expected bytes.".into(),
                        code: "invalid".into(),
                        params: HashMap::new(),
                    });
                }
            }
        }
    }
}

/// Counts total significant digits and decimal places from a Decimal's string
/// representation. Avoids parsing into `rust_decimal::Decimal` (which is ~200ns)
/// when we only need digit/scale counts for validation.
///
/// Handles: "50000.00" → (7, 2), "0.1" → (1, 1), "-123.45" → (5, 2),
/// "0" → (1, 0), "0.00" → (1, 2), "1E+2" → (3, 0)
fn count_decimal_digits(s: &str) -> (u32, u32) {
    let s = s.strip_prefix('-').unwrap_or(s);

    // Handle scientific notation (e.g., "1E+2", "1.5E-3")
    if let Some(e_pos) = s.find(['E', 'e']) {
        let mantissa_str = &s[..e_pos];
        let exp: i32 = s[e_pos + 1..].parse().unwrap_or(0);
        let (m_digits, m_scale) = count_decimal_digits(mantissa_str);
        let effective_scale = (m_scale as i32 - exp).max(0) as u32;
        let effective_digits = (m_digits as i32 + exp.max(0)).max(1) as u32;
        return (effective_digits, effective_scale);
    }

    if let Some(dot_pos) = s.find('.') {
        let int_part = &s[..dot_pos];
        let dec_part = &s[dot_pos + 1..];
        let scale = dec_part.len() as u32;

        // Count significant digits: all digits in mantissa (int_part + dec_part)
        // with leading zeros stripped, but at least 1.
        let combined: String = int_part.chars().chain(dec_part.chars()).collect();
        let significant = combined.trim_start_matches('0').len().max(1) as u32;

        // Match rust_decimal behavior: total digits = significant digits in mantissa
        // but trailing zeros in dec_part count (e.g., "50000.00" mantissa is 5000000 = 7 digits)
        let all_digits = combined.len().max(1) as u32;
        let leading_zeros = combined.len() as u32 - significant;
        let total = all_digits - leading_zeros;

        (total.max(1), scale)
    } else {
        // No decimal point
        let trimmed = s.trim_start_matches('0');
        let digits = trimmed.len().max(1) as u32;
        (digits, 0)
    }
}

// ─── Batch operations (Rayon-parallel, targeting Python's weaknesses) ────────

/// Validates a batch of person names in parallel.
///
/// For each name checks: max length, must contain a space, no digit characters,
/// only alphabetic/space/hyphen characters allowed. These character-by-character
/// scans are extremely expensive in Python (~7us per name) but trivial in Rust
/// (~50ns per name) because Rust operates on bytes directly with no per-char
/// object allocation.
///
/// Returns a list of dicts: `[{"valid": bool, "errors": [str]}, ...]`
#[pyfunction]
fn validate_names_batch<'py>(
    py: Python<'py>,
    names: &Bound<'py, PyList>,
) -> PyResult<Bound<'py, PyList>> {
    let k_valid = PyString::intern(py, "valid");
    let k_errors = PyString::intern(py, "errors");

    // Phase 1: extract all strings via zero-copy to_str() (GIL held).
    // Collect PyString refs first so they stay alive for the &str borrows.
    let py_strings: Vec<Bound<'_, PyAny>> = names.iter().collect();
    let rust_names: Vec<&str> = py_strings
        .iter()
        .map(|item| {
            let py_str = item.cast::<PyString>()?;
            py_str.to_str()
        })
        .collect::<PyResult<_>>()?;

    // Phase 2: validate in parallel (GIL released)
    let results: Vec<(bool, Vec<&str>)> = py.detach(|| {
        rust_names
            .par_iter()
            .map(|name| validate_name_rust(name))
            .collect()
    });

    // Phase 3: convert to Python (GIL held)
    let out = PyList::empty(py);
    for (valid, errors) in &results {
        let d = PyDict::new(py);
        d.set_item(&k_valid, *valid)?;
        let err_list = PyList::empty(py);
        for e in errors {
            err_list.append(PyString::intern(py, e))?;
        }
        d.set_item(&k_errors, err_list)?;
        out.append(d)?;
    }
    Ok(out)
}

/// Pure-Rust name validation — no Python object allocation per character.
fn validate_name_rust(name: &str) -> (bool, Vec<&'static str>) {
    let mut errors: Vec<&str> = Vec::new();

    if name.len() > 120 {
        errors.push("exceeds 120 characters");
    }

    let mut has_space = false;
    let mut has_digit = false;
    let mut has_invalid = false;
    for c in name.chars() {
        if c == ' ' {
            has_space = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        } else if c == '-' || c.is_alphabetic() {
            // ok
        } else {
            has_invalid = true;
        }
    }

    if !has_space {
        errors.push("must contain a space");
    }
    if has_digit {
        errors.push("must not contain digits");
    }
    if has_invalid {
        errors.push("contains invalid characters");
    }

    (errors.is_empty(), errors)
}

/// Validates a batch of national IDs against the pattern `^\d{8}-\d{4}$`.
///
/// Hand-written pattern matcher (no regex crate needed) — checks fixed
/// positions directly. Runs in parallel via Rayon.
///
/// Returns a list of booleans.
#[pyfunction]
fn validate_ids_batch<'py>(
    py: Python<'py>,
    ids: &Bound<'py, PyList>,
) -> PyResult<Bound<'py, PyList>> {
    // Zero-copy: borrow strings directly from Python's internal buffer
    let out = PyList::empty(py);
    for item in ids.iter() {
        let py_str = item.cast::<PyString>()?;
        let s = py_str.to_str()?;
        out.append(validate_national_id(s))?;
    }
    Ok(out)
}

/// Hand-written national ID validator: `^\d{8}-\d{4}$`
fn validate_national_id(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 13
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'-'
        && b[9..].iter().all(u8::is_ascii_digit)
}

/// Computes risk scores for a batch of records in parallel.
///
/// Extracts credit_score, debt_to_income_ratio, loan_amount, age, and
/// interest_rate from each dict, then runs the scoring formula across
/// all records using Rayon. The float math (log, sqrt, clamp) is ~20x
/// faster in Rust than CPython's interpreter loop.
///
/// Returns a list of floats.
#[pyfunction]
fn compute_risk_batch<'py>(
    py: Python<'py>,
    records: &Bound<'py, PyList>,
) -> PyResult<Bound<'py, PyList>> {
    // Pre-intern keys
    let k_credit = PyString::intern(py, "credit_score");
    let k_dti = PyString::intern(py, "debt_to_income_ratio");
    let k_loan = PyString::intern(py, "loan_amount");
    let k_age = PyString::intern(py, "age");
    let k_rate = PyString::intern(py, "interest_rate");

    // Phase 1: extract numeric fields (GIL held)
    let mut rows: Vec<(f64, f64, f64, f64, f64)> = Vec::with_capacity(records.len());
    for item in records.iter() {
        let dict = item.cast::<PyDict>()?;
        let credit: f64 = dict
            .get_item(&k_credit)?
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(500.0);
        let dti: f64 = dict
            .get_item(&k_dti)?
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(0.3);
        let loan: f64 = dict
            .get_item(&k_loan)?
            .map(|v| {
                v.extract::<f64>().or_else(|_| {
                    let s: String = v.str()?.extract()?;
                    Ok::<f64, clarax::PyErr>(s.parse::<f64>().unwrap_or(100_000.0))
                })
            })
            .transpose()?
            .unwrap_or(100_000.0);
        let age: f64 = dict
            .get_item(&k_age)?
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(30.0);
        let rate: f64 = dict
            .get_item(&k_rate)?
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(5.0);
        rows.push((credit, dti, loan, age, rate));
    }

    // Phase 2: compute scores in parallel (GIL released)
    let scores: Vec<f64> = py.detach(|| {
        rows.par_iter()
            .map(|&(credit, dti, loan, age, rate)| {
                let score: f64 = 100.0 - credit / 10.0 + dti * 50.0
                    + (1.0_f64 + loan).ln() * 2.0
                    - if age >= 25.0 { 5.0 } else { 0.0 }
                    + rate * 0.5
                    - (credit - 300.0_f64).max(0.0).sqrt() * 0.3;
                score.clamp(0.0, 100.0)
            })
            .collect()
    });

    // Phase 3: convert to Python list (GIL held)
    let out = PyList::empty(py);
    for s in &scores {
        out.append(*s)?;
    }
    Ok(out)
}

/// Computes batch statistics (mean, median, stdev, min, max) in Rust.
///
/// Python's `statistics` module is pure Python and slow for large lists.
/// This uses a single parallel pass for sum/sum_of_squares (Rayon),
/// then a sort for median.
///
/// Returns a dict with mean, median, stdev, min, max, count.
#[pyfunction]
fn batch_stats<'py>(
    py: Python<'py>,
    values: &Bound<'py, PyList>,
) -> PyResult<Bound<'py, PyDict>> {
    let mut data: Vec<f64> = values
        .iter()
        .map(|v| v.extract::<f64>())
        .collect::<PyResult<_>>()?;

    let n = data.len();
    if n == 0 {
        let d = PyDict::new(py);
        d.set_item("count", 0)?;
        return Ok(d);
    }

    // Parallel sum and sum-of-squares
    let (sum, sum_sq, min_val, max_val) = py.detach(|| {
        data.par_iter().fold(
            || (0.0_f64, 0.0_f64, f64::INFINITY, f64::NEG_INFINITY),
            |(s, ss, mn, mx): (f64, f64, f64, f64), &v| {
                (s + v, ss + v * v, mn.min(v), mx.max(v))
            },
        ).reduce(
            || (0.0_f64, 0.0_f64, f64::INFINITY, f64::NEG_INFINITY),
            |(s1, ss1, mn1, mx1): (f64, f64, f64, f64),
             (s2, ss2, mn2, mx2): (f64, f64, f64, f64)| {
                (s1 + s2, ss1 + ss2, mn1.min(mn2), mx1.max(mx2))
            },
        )
    });

    let mean = sum / n as f64;
    let variance: f64 = (sum_sq / n as f64) - mean * mean;
    let stdev = variance.max(0.0).sqrt();

    // Sort for median (parallel sort)
    py.detach(|| data.par_sort_unstable_by(|a: &f64, b: &f64| a.partial_cmp(b).unwrap()));
    let median = if n % 2 == 0 {
        (data[n / 2 - 1] + data[n / 2]) / 2.0
    } else {
        data[n / 2]
    };

    let d = PyDict::new(py);
    d.set_item(PyString::intern(py, "mean"), mean)?;
    d.set_item(PyString::intern(py, "median"), median)?;
    d.set_item(PyString::intern(py, "stdev"), stdev)?;
    d.set_item(PyString::intern(py, "min"), min_val)?;
    d.set_item(PyString::intern(py, "max"), max_val)?;
    d.set_item(PyString::intern(py, "count"), n)?;
    Ok(d)
}

/// Returns the clarax-core version string.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The native extension module.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Schema>()?;
    m.add_class::<Field>()?;
    m.add_function(wrap_pyfunction!(serialize, m)?)?;
    m.add_function(wrap_pyfunction!(serialize_many, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(validate_many, m)?)?;
    m.add_function(wrap_pyfunction!(validate_names_batch, m)?)?;
    m.add_function(wrap_pyfunction!(validate_ids_batch, m)?)?;
    m.add_function(wrap_pyfunction!(compute_risk_batch, m)?)?;
    m.add_function(wrap_pyfunction!(batch_stats, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}

// ─── Value extraction from Python ─────────────────────────────────────────────

/// Extracts all field values from a Python dict or object.
fn extract_values<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
    descriptors: &[FieldDescriptor],
) -> PyResult<Vec<FieldValue>> {
    let mut values = Vec::with_capacity(descriptors.len());
    for desc in descriptors {
        values.push(extract_single_value(py, data, desc)?);
    }
    Ok(values)
}

/// Extracts a single field value from a Python dict or object.
fn extract_single_value<'py>(
    _py: Python<'py>,
    data: &Bound<'py, PyAny>,
    desc: &FieldDescriptor,
) -> PyResult<FieldValue> {
    // Try dict access first, then attribute access
    let py_val = if let Ok(dict) = data.cast::<PyDict>() {
        dict.get_item(&desc.name)?
    } else {
        data.getattr(desc.name.as_str()).ok()
    };

    let py_val = match py_val {
        Some(v) if !v.is_none() => v,
        _ => {
            if desc.nullable || desc.has_default {
                return Ok(FieldValue::Null);
            }
            return Err(CoreError::NullField {
                field: desc.name.clone(),
            }
            .into());
        }
    };

    convert_py_to_field_value(&py_val, desc)
}

/// Converts a Python object to a FieldValue based on the field descriptor.
fn convert_py_to_field_value(
    val: &Bound<'_, PyAny>,
    desc: &FieldDescriptor,
) -> PyResult<FieldValue> {
    match &desc.field_type {
        FieldType::Str { .. } => {
            let s: String = val.extract()?;
            Ok(FieldValue::Text(s))
        }
        FieldType::Int { .. } => {
            let n: i64 = val.extract()?;
            Ok(FieldValue::Integer(n))
        }
        FieldType::Float { .. } => {
            let f: f64 = val.extract()?;
            Ok(FieldValue::Float(f))
        }
        FieldType::Bool => {
            let b: bool = val.extract()?;
            Ok(FieldValue::Boolean(b))
        }
        FieldType::Decimal { .. } => {
            // Python Decimal → string → rust_decimal::Decimal
            let s: String = val.str()?.extract()?;
            let d = Decimal::from_str_exact(&s).map_err(|e| CoreError::TypeError {
                field: desc.name.clone(),
                expected: "Decimal".into(),
                got: format!("invalid decimal string: {e}"),
            })?;
            Ok(FieldValue::Decimal(d))
        }
        FieldType::DateTime => {
            let iso: String = val.call_method0("isoformat")?.extract()?;
            let dt: DateTime<Utc> = iso
                .parse()
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&iso, "%Y-%m-%dT%H:%M:%S%.f")
                        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&iso, "%Y-%m-%dT%H:%M:%S"))
                        .map(|ndt| ndt.and_utc())
                })
                .map_err(|e| CoreError::TypeError {
                    field: desc.name.clone(),
                    expected: "datetime".into(),
                    got: format!("could not parse: {e}"),
                })?;
            Ok(FieldValue::DateTime(dt))
        }
        FieldType::Date => {
            let iso: String = val.call_method0("isoformat")?.extract()?;
            let d = NaiveDate::parse_from_str(&iso, "%Y-%m-%d").map_err(|e| {
                CoreError::TypeError {
                    field: desc.name.clone(),
                    expected: "date".into(),
                    got: format!("could not parse: {e}"),
                }
            })?;
            Ok(FieldValue::Date(d))
        }
        FieldType::Time => {
            let iso: String = val.call_method0("isoformat")?.extract()?;
            let t = NaiveTime::parse_from_str(&iso, "%H:%M:%S%.f")
                .or_else(|_| NaiveTime::parse_from_str(&iso, "%H:%M:%S"))
                .map_err(|e| CoreError::TypeError {
                    field: desc.name.clone(),
                    expected: "time".into(),
                    got: format!("could not parse: {e}"),
                })?;
            Ok(FieldValue::Time(t))
        }
        FieldType::Uuid => {
            let s: String = val.str()?.extract()?;
            let u = Uuid::parse_str(&s).map_err(|e| CoreError::TypeError {
                field: desc.name.clone(),
                expected: "UUID".into(),
                got: format!("invalid UUID: {e}"),
            })?;
            Ok(FieldValue::Uuid(u))
        }
        FieldType::List | FieldType::Dict => {
            // Convert Python list/dict → JSON via str(json.dumps())
            let json_str: String = {
                let json_mod = val.py().import("json")?;
                let dumped = json_mod.call_method1("dumps", (val,))?;
                dumped.extract()?
            };
            let json_val: serde_json::Value =
                serde_json::from_str(&json_str).map_err(|e| CoreError::TypeError {
                    field: desc.name.clone(),
                    expected: desc.field_type.type_name().into(),
                    got: format!("invalid JSON: {e}"),
                })?;
            Ok(FieldValue::Json(json_val))
        }
        FieldType::Bytes { .. } => {
            let b: Vec<u8> = val.extract()?;
            Ok(FieldValue::Binary(b))
        }
    }
}

// ─── Output conversion helpers ────────────────────────────────────────────────

/// Converts a serialized record to a Python dict.
fn record_to_pydict<'py>(
    py: Python<'py>,
    record: &serde_json::Map<String, serde_json::Value>,
) -> PyResult<Bound<'py, PyDict>> {
    let output = PyDict::new(py);
    for (key, val) in record {
        let py_val = json_to_py(py, val)?;
        output.set_item(key, py_val)?;
    }
    Ok(output)
}

/// Converts a ValidationReport to a Python dict.
fn report_to_pydict<'py>(
    py: Python<'py>,
    report: &ValidationReport,
) -> PyResult<Bound<'py, PyDict>> {
    let result = PyDict::new(py);
    result.set_item("valid_count", report.valid_count)?;
    result.set_item("error_count", report.error_count)?;
    result.set_item("is_valid", report.is_valid())?;

    let errors = PyList::empty(py);
    for err in &report.field_errors {
        let err_dict = PyDict::new(py);
        err_dict.set_item("field", &err.field_name)?;
        err_dict.set_item("message", &err.message)?;
        err_dict.set_item("code", &err.code)?;
        let params = PyDict::new(py);
        for (k, v) in &err.params {
            params.set_item(k, v)?;
        }
        err_dict.set_item("params", params)?;
        errors.append(err_dict)?;
    }
    result.set_item("errors", errors)?;
    Ok(result)
}

/// Converts a serde_json::Value to a Python object.
fn json_to_py<'py>(py: Python<'py>, value: &serde_json::Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None().into_bound(py)),
        serde_json::Value::Bool(b) => Ok(PyBool::new(py, *b).to_owned().into_any()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(PyInt::new(py, i).into_any())
            } else if let Some(f) = n.as_f64() {
                Ok(PyFloat::new(py, f).into_any())
            } else {
                Ok(PyString::new(py, &n.to_string()).into_any())
            }
        }
        serde_json::Value::String(s) => Ok(PyString::new(py, s).into_any()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.into_any())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into_any())
        }
    }
}
