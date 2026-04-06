# clarax-core — Technical Plan

**Author:** Abdulwahed Mansour
**Version:** v1.0.1
**Status:** Published on PyPI and crates.io

---

## What is clarax-core?

clarax-core is a Rust-accelerated serialization and validation engine for Python. It takes plain Python dicts or objects, converts them through a precompiled schema of typed fields with constraints, and returns validated, JSON-compatible output.

No framework required. Works with Flask, FastAPI, scripts, ETL pipelines, or any Python code that processes structured data. It is the framework-agnostic foundation that clarax-django delegates to.

---

## Public API

### Schema-based operations

```python
from clarax_core import Schema, Field, serialize, serialize_many, validate, validate_many

schema = Schema({
    "name":  Field(str, max_length=100),
    "age":   Field(int, min_value=0, max_value=150),
    "price": Field(Decimal, max_digits=10, decimal_places=2),
    "joined": Field(datetime),
    "active": Field(bool),
    "id":    Field(UUID),
})

# Single record
result = serialize({"name": "Erik", "age": 30, ...}, schema)
report = validate({"name": "Erik", "age": -5, ...}, schema)

# Batch (optimized: dict.copy + selective overwrite)
results = serialize_many(data_list, schema)
report  = validate_many(data_list, schema)
```

### Auto-schema generation

```python
from clarax_core import from_dataclass, from_typeddict

@dataclass
class User:
    name: str
    age: int

schema = from_dataclass(User)
```

### Batch operations (Rayon-parallel)

```python
from clarax_core import validate_names_batch, validate_ids_batch
from clarax_core import compute_risk_batch, batch_stats

# Character scanning — 9x over Python
validate_names_batch(["Erik Andersson", "Bad123 Name"])

# Pattern matching — 15x over Python
validate_ids_batch(["19900515-1234", "invalid"])

# Parallel float math — 3.7x over Python
compute_risk_batch(records)

# Parallel statistics — 20x over Python
batch_stats([1.0, 2.0, 3.0, 4.0, 5.0])
```

---

## Supported Types

| Type | Constraints |
|---|---|
| `str` | `max_length`, `min_length` |
| `int` | `min_value`, `max_value` |
| `float` | `min_value`, `max_value` |
| `bool` | -- |
| `Decimal` | `max_digits`, `decimal_places` |
| `datetime` | -- |
| `date` | -- |
| `time` | -- |
| `UUID` | -- |
| `list` | -- |
| `dict` | -- |
| `bytes` | `max_length` |

All types accept `nullable=True` and `default=True`.

Invalid constraints raise `SchemaError` at definition time: `Field(int, max_length=100)` fails immediately, not at runtime.

---

## Performance

| Operation | Python | ClaraX | Speedup |
|---|---|---|---|
| Name validation (150K) | 2,147 ms | 237 ms | **9.1x** |
| Pattern matching (50K) | 46 ms | 3 ms | **15.5x** |
| Batch statistics (50K) | 96 ms | 5 ms | **20.8x** |
| Dict serialization (50K) | 284 ms | 127 ms | **2.2x** |

Python 3.14t (free-threading) supported with zero code changes.

---

## Relationship to clarax-django

clarax-django depends on clarax-core. All serialization and validation logic lives in clarax-core. clarax-django adds:

- `ModelSchema` — auto-generates Schema from Django model `_meta`
- `RustSerializerMixin` — drop-in DRF mixin
- `serialize_values_list()` — direct queryset serialization
- `clarax_doctor` — management command to audit serializers
