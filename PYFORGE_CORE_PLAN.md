# pyforge-core — Technical Plan

**Author:** Abdulwahed Mansour
**Version:** v0.2.0
**Status:** Draft — awaiting approval

---

## 1a. What is pyforge-core?

pyforge-core is a Rust-accelerated serialization and validation engine for Python. It takes plain Python dicts or objects, converts them through a precompiled schema of typed fields with constraints, and returns validated, JSON-compatible output — all in Rust. It requires no framework. A Flask developer, a FastAPI developer, a data engineer writing ETL scripts, or anyone processing structured data in Python can use it. It is the framework-agnostic foundation that pyforge-django will delegate to.

---

## 1b. Public API Surface

```python
from pyforge_core import Schema, Field, serialize, serialize_many, validate, validate_many, version

# Schema definition
schema = Schema({
    "name": Field(str, max_length=100),
    "age": Field(int, min_val=0, max_val=150),
    "email": Field(str, max_length=254),
    "salary": Field(Decimal, max_digits=10, decimal_places=2),
    "joined": Field(datetime),
    "active": Field(bool),
    "tags": Field(list),
    "metadata": Field(dict),
    "id": Field(UUID),
})

# Serialize a dict
result = serialize(data_dict, schema)           # → dict

# Serialize a list of dicts
results = serialize_many(list_of_dicts, schema)  # → list[dict]

# Validate a dict
report = validate(data_dict, schema)             # → {"is_valid": bool, "errors": [...]}

# Validate a list of dicts
report = validate_many(list_of_dicts, schema)    # → {"is_valid": bool, "errors": [...]}

# Serialize a Python object (any object with attributes)
result = serialize(my_obj, schema)               # → dict (uses getattr internally)
```

### Naming justification

| Name | Chosen | Rejected | Why |
|------|--------|----------|-----|
| `Schema` | Yes | `TypeMap`, `Blueprint`, `Spec`, `Serializer` | "Schema" is the universal term (pydantic, marshmallow, JSON Schema, GraphQL). `Serializer` conflicts with DRF's concept. `TypeMap` and `Blueprint` are invented terms nobody searches for. |
| `Field` | Yes | `Col`, `Attr`, `Param`, `Type` | "Field" matches Django, pydantic, marshmallow, dataclasses. Universal. `Type` shadows Python's builtin. |
| `serialize` | Yes | `dump`, `encode`, `convert`, `to_dict` | "serialize" matches DRF, marshmallow, and general industry usage. `dump` is marshmallow-specific. `encode` implies encoding format (JSON, msgpack) not structure. |
| `serialize_many` | Yes | `serialize_batch`, `serialize_list`, `bulk_serialize` | "many" mirrors DRF's `Serializer(many=True)` — Django developers know this pattern. `batch` is used by pyforge-django internally but `many` is the user-facing convention. |
| `validate` | Yes | `check`, `verify`, `is_valid` | "validate" is the standard. `check` is too vague. `is_valid` implies a boolean — our validate returns a full report. |
| `validate_many` | Yes | `validate_batch`, `bulk_validate` | Consistent with `serialize_many`. |

---

## 1c. Supported Field Types

| Python type | Field definition | Rust type | Constraints |
|---|---|---|---|
| `str` | `Field(str)` | `String` | `max_length`, `min_length` |
| `int` | `Field(int)` | `i64` | `min_val`, `max_val` |
| `float` | `Field(float)` | `f64` | `min_val`, `max_val` |
| `bool` | `Field(bool)` | `bool` | (none) |
| `Decimal` | `Field(Decimal)` | `rust_decimal::Decimal` | `max_digits`, `decimal_places` |
| `datetime` | `Field(datetime)` | `chrono::DateTime<Utc>` | (none) |
| `date` | `Field(date)` | `chrono::NaiveDate` | (none) |
| `time` | `Field(time)` | `chrono::NaiveTime` | (none) |
| `UUID` | `Field(UUID)` | `uuid::Uuid` | (none) |
| `list` | `Field(list)` | `serde_json::Value` | (none — stored as JSON array) |
| `dict` | `Field(dict)` | `serde_json::Value` | (none — stored as JSON object) |
| `bytes` | `Field(bytes)` | `Vec<u8>` | `max_length` |
| `None`-able | `Field(str, nullable=True)` | `Option<String>` | (wraps any type) |

All types accept `nullable=True` and `default=...` as universal constraints.

---

## 1d. Schema Definition Approach

**Chosen: Explicit dict-based definition (`Schema({...})`)**

```python
schema = Schema({
    "name": Field(str, max_length=100),
    "age": Field(int, min_val=0, max_val=150),
    "email": Field(str, nullable=True),
})
```

**Rejected: Decorator-based class definition**

```python
@pyforge.schema
class UserSchema:
    name: str
    age: int
```

**Reasons:**

1. **Constraints need somewhere to live.** Type hints alone cannot express `max_length=100`. You'd need `Annotated[str, MaxLength(100)]` which is verbose and unfamiliar, or `name: str = Field(max_length=100)` which reintroduces Field anyway. The dict approach is equivalent but without the class boilerplate.

2. **Schemas must be sendable to Rust at construction time.** The dict approach compiles the schema in `Schema.__init__()` — one Rust call, done. A decorator would need to inspect `__annotations__` and class attributes, reconstruct the same dict internally, then call Rust. Extra Python work for no user benefit.

3. **Runtime schema generation is common.** API frameworks often build schemas from database introspection, config files, or user input. `Schema(field_dict)` works naturally with `dict(...)` or comprehensions. A class-based approach would require `type()` metaclass tricks.

4. **pyforge-django already works this way internally.** `ModelSchema` builds field descriptors from a dict-like structure (`_meta.get_fields()`). Making pyforge-core dict-based means pyforge-django's migration is a thin wrapper, not a rewrite.

**Future option:** A `@schema` decorator can be added later as syntactic sugar that calls `Schema({})` internally. The dict API is the foundation.

---

## 1e. Dependency Graph

```
┌─────────────────────────────────────────────────┐
│                  Python user code                │
│        (Flask, FastAPI, scripts, ETL, etc.)       │
└────────────┬────────────────────────┬────────────┘
             │                        │
             ▼                        ▼
┌────────────────────┐   ┌─────────────────────────┐
│   pyforge-core     │   │   pyforge-django         │
│   (pip install     │   │   (pip install            │
│    pyforge-core)   │   │    pyforge-django)        │
│                    │   │                           │
│   Schema, Field,   │   │   ModelSchema,            │
│   serialize,       │   │   serialize_instance,     │
│   validate         │   │   RustSerializerMixin     │
└────────┬───────────┘   └──────────┬────────────────┘
         │                          │
         │              ┌───────────┘
         ▼              ▼
┌────────────────────────────────────┐
│   pyforge-core (Rust crate)        │
│                                    │
│   FieldType, FieldDescriptor,      │
│   FieldValue, serialize_fields(),  │
│   validate_field_batch()           │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│   pyforge (Rust crate)             │
│   (PyO3 fork — Rust↔Python bridge) │
└────────────────────────────────────┘
```

Key constraints:
- pyforge-core MUST NOT import from pyforge-django
- pyforge-django DEPENDS ON pyforge-core (after refactor)
- pyforge-core has zero Django knowledge — no `_meta`, no `Model`, no `QuerySet`
- The existing `pyforge-django` public API does not change

---

## 1f. Rayon Parallel Threshold

**Threshold: 128 entries** (up from pyforge-django's 64).

Reasoning: Without Django's field descriptor overhead (which adds ~200ns per field extraction via `_meta`), each field processes faster in pyforge-core. The crossover point where Rayon's thread pool dispatch overhead (~2-5μs) is recovered by parallelism is therefore higher. With pyforge-core processing raw dict values (no `getattr` needed for dicts), single-field validation takes ~50-100ns. At 128 entries, the sequential path takes ~6-12μs — just above Rayon's dispatch cost. Below 128, serial is faster.

This should be validated by benchmarking after implementation and adjusted if needed.

---

## 1g. Three Benchmark Scenarios

**Scenario 1: API response serialization (Flask/FastAPI equivalent)**
Serialize 1,000 user records (8 fields: str, int, Decimal, datetime, UUID, bool, str, int) from a list of dicts. This models the most common use case: building a JSON API response from database rows.

**Scenario 2: CSV/ETL validation**
Validate 10,000 rows (12 fields each) with 5% intentional errors. This models data import validation — parsing a CSV, checking constraints, collecting errors. The 120,000 total field validations should trigger Rayon parallelism.

**Scenario 3: Single-record round-trip**
Serialize + validate one record with 20 fields. This measures per-call overhead — important for microservices processing one request at a time. The target is <50μs total.

---

## 2. Crate Structure

```
pyforge/
├── pyforge-core/                    ← NEW CRATE
│   ├── Cargo.toml
│   ├── pyproject.toml               ← maturin config for PyPI
│   ├── README.md
│   ├── src/
│   │   ├── lib.rs                   ← #[pymodule] + Python-exposed API
│   │   ├── field_types.rs           ← FieldType, FieldDescriptor, FieldValue (extracted from pyforge-django)
│   │   ├── serializer.rs            ← serialize_fields(), serialize_batch() (extracted)
│   │   ├── validator.rs             ← validate_field_batch() with Rayon (extracted)
│   │   └── error.rs                 ← CoreError type
│   └── python/
│       └── pyforge_core/
│           ├── __init__.py          ← Schema, Field, serialize, validate
│           └── __init__.pyi         ← Type stubs
├── pyforge-django/                  ← EXISTING — refactored to depend on pyforge-core
│   ├── Cargo.toml                   ← adds dependency: pyforge-core = { path = "../pyforge-core" }
│   ├── src/
│   │   ├── lib.rs                   ← ModelSchema stays here (Django-specific)
│   │   ├── model.rs                 ← Django _meta extraction stays here
│   │   ├── field_types.rs           ← REMOVED — uses pyforge-core's types
│   │   ├── serializer.rs            ← REMOVED — delegates to pyforge-core
│   │   ├── validator.rs             ← REMOVED — delegates to pyforge-core
│   │   └── ...
```

### Cargo.toml changes

Root `Cargo.toml` workspace members:
```toml
members = [
    # ... existing members ...
    "pyforge-core",
]
```

`pyforge-core/Cargo.toml`:
```toml
[package]
name = "pyforge-core"
version = "0.2.0"

[dependencies]
pyforge = { path = "..", version = "0.1.0", features = ["macros"] }
chrono = { version = "0.4.25", features = ["serde"] }
rust_decimal = { version = "1.15", features = ["serde-with-str"] }
uuid = { version = "1.12.0", features = ["v4", "serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rayon = "1.6"
thiserror = "2"
```

`pyforge-django/Cargo.toml` (after refactor):
```toml
[dependencies]
pyforge-core = { path = "../pyforge-core", version = "0.2.0" }
pyforge = { path = "..", version = "0.1.0", features = ["macros"] }
# chrono, rust_decimal, uuid, serde, rayon — REMOVED (come transitively from pyforge-core)
```

---

## 3. Migration Plan for pyforge-django

### What moves to pyforge-core:

| File | What moves | What stays in pyforge-django |
|---|---|---|
| `field_types.rs` | `FieldType` (renamed from `DjangoFieldType`), `FieldDescriptor`, `FieldValue` — entire file | `DjangoFieldType` becomes a thin wrapper that maps Django type names to `pyforge_core::FieldType` |
| `serializer.rs` | `serialize_model_fields()`, `serialize_queryset_rows()`, `field_value_to_json()`, base64 encoder — entire file | `serialize_instance()` stays (it does `getattr` extraction, which is Django-specific) |
| `validator.rs` | `validate_field_batch()`, `validate_single_field()`, `ValidationReport` — entire file | `validate_instance()` stays (it does `getattr` extraction) |
| `error.rs` | `FieldValidationError` struct | `DjangoError` stays (has Django-specific error variants) |

### What stays in pyforge-django (does NOT move):

- `ModelSchema` — compiles from Django `_meta` API
- `model.rs` — `extract_field_descriptors()` reads Django model introspection
- `lib.rs` — `serialize_instance()` and `validate_instance()` do `getattr` on Django model instances
- `async_bridge.rs` — Django-specific GIL release patterns
- `django_pyforge/serializers.py` — `RustSerializerMixin` (DRF-specific)
- All Python files in `django_pyforge/`

### What MUST NOT change:

- `from django_pyforge import ModelSchema, serialize_instance, validate_instance` — same imports
- `ModelSchema(MyModel)` — same constructor
- `serialize_instance(obj, schema)` — same signature, same return format
- `RustSerializerMixin` — same drop-in mixin behavior
- All 59 Hyra tests must pass without modification

---

## 4. Naming Decision Table

| Concept | Chosen name | Rejected alternatives | Reason |
|---|---|---|---|
| Main schema class | `Schema` | `TypeMap`, `Blueprint`, `Spec`, `Serializer`, `Model` | Universal term. `Serializer` conflicts with DRF. `Model` conflicts with Django/SQLAlchemy. |
| Field definition | `Field` | `Col`, `Attr`, `Param`, `FieldDef` | Matches Django, pydantic, marshmallow, dataclasses. One syllable, universal. |
| Serialize one | `serialize` | `dump`, `encode`, `convert`, `to_dict` | Industry standard. `dump` is marshmallow-specific. `encode` implies format. |
| Serialize batch | `serialize_many` | `serialize_batch`, `serialize_list`, `bulk_serialize` | `many` is the DRF convention (`Serializer(many=True)`). Consistent across the ecosystem. |
| Validate one | `validate` | `check`, `verify`, `is_valid` | Standard term. `is_valid` implies boolean return. |
| Validate batch | `validate_many` | `validate_batch`, `bulk_validate` | Consistent with `serialize_many`. |
| Rust field type enum | `FieldType` | `DjangoFieldType`, `ValueType`, `DataType` | Framework-agnostic. `DjangoFieldType` stays in pyforge-django as a wrapper. |
| Rust field value enum | `FieldValue` | `Value`, `TypedValue`, `RustValue` | Same name — no reason to change. Clear enough. |
| Python package | `pyforge-core` | `pyforge`, `pyforgecore`, `pyforge_core` | `pyforge` on PyPI could be confused with the Rust binding crate on crates.io. `pyforge-core` is explicit. Hyphenated for PyPI, underscored for import (`import pyforge_core`). |
| Rust crate | `pyforge-core` | `pyforge-engine`, `pyforge-base`, `pyforge-lib` | "core" is the standard suffix for the foundation crate (tokio-core, actix-core, etc.). |

---

## 5. Publication Plan

| Platform | Name | URL |
|---|---|---|
| crates.io | `pyforge-core` | `https://crates.io/crates/pyforge-core` |
| PyPI | `pyforge-core` | `https://pypi.org/project/pyforge-core/` |

### Publish order (dependencies first):
1. `pyforge-core` v0.2.0 to **crates.io**
2. `pyforge-core` v0.2.0 to **PyPI** (wheel build via publish-pypi.sh)
3. `pyforge-django` v0.2.0 to **crates.io** (updated dependency)
4. `pyforge-django` v0.2.0 to **PyPI** (updated wheel)

### Documentation:
- `pyforge-core/README.md` — standalone README for PyPI page (installation, quickstart, benchmarks)
- No separate docs site yet — README + docstrings are sufficient for v0.2.0
- pyforge-django README updated to mention pyforge-core as the engine

---

## 6. Estimated Test Count

### pyforge-core Rust unit tests (~35)

| Category | Count | Covers |
|---|---|---|
| FieldType/FieldValue | 5 | Type names, serialization, equality |
| Serializer | 10 | Each field type → JSON, null handling, errors, batch |
| Validator | 15 | Null, type mismatch, max_length, decimal digits, slug, binary, parallel ordering |
| Error | 3 | Error display, code strings, params |
| Schema compilation | 2 | Valid schema, invalid schema |

### pyforge-core Python integration tests (~20)

| Category | Count | Covers |
|---|---|---|
| Schema + Field | 5 | Construction, field listing, repr, invalid types |
| serialize | 5 | Dict input, object input, null handling, type coercion, error cases |
| serialize_many | 3 | Empty list, 100 items, mixed valid/invalid |
| validate | 4 | All-valid, all-invalid, mixed, constraint violations |
| validate_many | 3 | Empty, large batch (Rayon), error ordering |

### Benchmark tests (~3)

| Scenario | What it measures |
|---|---|
| API response (1K records, 8 fields) | serialize_many throughput |
| ETL validation (10K rows, 12 fields) | validate_many with Rayon |
| Single-record round-trip (1 record, 20 fields) | Per-call overhead |

### Total: ~58 new tests

### Existing tests that must still pass:
- pyforge workspace: 1,279 tests
- pyforge-django: 33 Rust tests (may move to pyforge-core)
- Hyra: 59 Django tests

---

*This plan is the authoritative specification for pyforge-core v0.2.0.*
*No code will be written until the plan is approved.*
