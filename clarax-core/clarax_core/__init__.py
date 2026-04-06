# Author: Abdulwahed Mansour
"""
clarax-core — Rust-accelerated serialization and validation for Python.

Framework-agnostic. Works with Flask, FastAPI, scripts, ETL pipelines,
or any Python code that processes structured data.

Usage:
    from clarax_core import Schema, Field, serialize, serialize_many, validate, validate_many
"""

__author__ = "Abdulwahed Mansour"
__version__ = "1.0.1"

from clarax_core._native import (
    Schema,
    Field,
    serialize,
    serialize_many,
    validate,
    validate_many,
    validate_names_batch,
    validate_ids_batch,
    compute_risk_batch,
    batch_stats,
    version,
)

from clarax_core.auto_schema import from_dataclass, from_typeddict
from clarax_core.constraints import (
    DecimalPlaces,
    MaxDigits,
    MaxLength,
    MaxValue,
    MinLength,
    MinValue,
)

__all__ = [
    "Schema",
    "Field",
    "serialize",
    "serialize_many",
    "validate",
    "validate_many",
    "validate_names_batch",
    "validate_ids_batch",
    "compute_risk_batch",
    "batch_stats",
    "version",
    "from_dataclass",
    "from_typeddict",
    "MaxLength",
    "MinLength",
    "MinValue",
    "MaxValue",
    "MaxDigits",
    "DecimalPlaces",
]
