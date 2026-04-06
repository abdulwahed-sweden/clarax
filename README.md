<p align="center">
  <img src="assets/logo-full.png" alt="ClaraX" width="420">
</p>

<p align="center">
  <strong>Rust-accelerated serialization and validation for Django.</strong><br>
  Drop-in for Django REST Framework. Standalone for everything else.
</p>

<p align="center">
  <a href="https://pypi.org/project/clarax-django"><img src="https://img.shields.io/pypi/v/clarax-django.svg" alt="PyPI django"></a>
  <a href="https://pypi.org/project/clarax-core"><img src="https://img.shields.io/pypi/v/clarax-core.svg" alt="PyPI core"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.python.org"><img src="https://img.shields.io/badge/python-3.11%2B-blue.svg" alt="Python 3.11+"></a>
  <a href="https://www.djangoproject.com"><img src="https://img.shields.io/badge/django-4.2%2B-green.svg" alt="Django 4.2+"></a>
</p>

---

## When to Use ClaraX

ClaraX replaces **DRF's serialization and validation overhead**, not Python itself.
Use this decision guide:

### Use ClaraX (Django)

| Scenario | Expected Speedup | Why |
|---|---|---|
| List API returning 50+ records | **2-3x** over DRF | Rust bypasses DRF's per-field Python dispatch |
| Bulk create/update with validation | **2-3x** over DRF | Batch validation in single Rust call |
| Export endpoint (1K+ records) | **3.5x** via `serialize_values_list()` | Single Rust call for entire queryset |
| Batch name/string validation | **9x** over Python | Character scanning without per-char object allocation |
| Pattern matching (regex-like) | **15x** over Python | Hand-written Rust byte matcher vs Python `re` |
| Batch statistics (mean/median/stdev) | **20x** over Python `statistics` | Rayon parallel reduce + sort |

### Do NOT Use ClaraX

| Scenario | Why |
|---|---|
| Single-record detail views | Bridge overhead (~10us) exceeds DRF overhead for 1 record |
| Database-bound views | ClaraX does not touch query time. Optimize your queries first |
| Raw `.values()` dict comprehensions | Already C-speed. ClaraX replaces DRF, not CPython's dict ops |
| Views with `SerializerMethodField` everywhere | Python-computed fields bypass Rust entirely |
| Simple apps with <100 req/s | DRF is fast enough. Don't add complexity you don't need |

### Quick Check

```bash
# Run on your Django project — tells you exactly which serializers benefit
python manage.py clarax_doctor --json
```

---

## Performance

### Django (clarax-django vs DRF ModelSerializer)

Measured on a 17-field model, CPython 3.12, SQLite:

| Method | 500 records | vs DRF |
|---|---|---|
| Pure DRF ModelSerializer | 57 ms | baseline |
| DRF + `RustSerializerMixin` | 26 ms | **2.2x** |
| `serialize_batch()` | 21 ms | **2.7x** |

### Standalone (clarax-core, 50K records)

| Operation | Python | ClaraX | Speedup |
|---|---|---|---|
| Name validation (150K names) | 2,147 ms | 237 ms | **9.1x** |
| Pattern matching (50K IDs) | 46 ms | 3 ms | **15.5x** |
| Risk computation (50K records) | 238 ms | 64 ms | **3.7x** |
| Batch statistics (50K values) | 96 ms | 5 ms | **20.8x** |
| Dict serialization (50K x 11 fields) | 284 ms | 127 ms | **2.2x** |

### Python 3.14 Free-Threading

ClaraX supports Python 3.14t (no-GIL) with zero code changes.
Rayon parallel operations use all CPU cores:

| Operation | 3.12 (GIL) | 3.14t (no-GIL) | Change |
|---|---|---|---|
| serialize_many | 54 ms | 47 ms | **13% faster** |
| validate_names | 106 ms | 77 ms | **27% faster** |

---

## Install

```bash
pip install clarax-django        # Django projects
pip install clarax-core           # Any Python project (Flask, FastAPI, scripts)
```

Add to `INSTALLED_APPS`:

```python
INSTALLED_APPS = [
    ...
    "django_clarax",
]
```

## Django Quickstart

### Step 1: Add the mixin (one line)

```python
from django_clarax.serializers import RustSerializerMixin

class MySerializer(RustSerializerMixin, serializers.ModelSerializer):
    class Meta:
        model = MyModel
        fields = "__all__"
```

### Step 2: Check your project

```bash
python manage.py clarax_doctor
```

Output tells you exactly which serializers benefit and which fields are Rust-accelerated.

### Step 3: Use batch APIs for bulk operations

```python
from django_clarax import ModelSchema, serialize_batch, serialize_values_list

schema = ModelSchema(MyModel)

# From model instances (2.7x over DRF)
results = serialize_batch(queryset, schema)

# From values_list (3.5x over DRF, lowest overhead)
results = serialize_values_list(queryset, schema)

# Streaming for large exports (constant memory)
for chunk in serialize_stream(queryset, schema, chunk_size=500):
    yield chunk
```

## Standalone Quickstart (clarax-core)

```python
from clarax_core import Schema, Field, serialize_many, validate_many
from decimal import Decimal

schema = Schema({
    "name": Field(str, max_length=100),
    "age":  Field(int, min_value=0, max_value=150),
    "price": Field(Decimal, max_digits=10, decimal_places=2),
})

data = [{"name": "Erik", "age": 30, "price": Decimal("199.99")}]
serialized = serialize_many(data, schema)
report = validate_many(data, schema)
```

### Batch operations (where Rust dominates)

```python
from clarax_core import validate_names_batch, validate_ids_batch, batch_stats

# Character scanning — 9x over Python
results = validate_names_batch(["Erik Andersson", "Bad123"])

# Pattern matching — 15x over Python
valid = validate_ids_batch(["19900515-1234", "invalid"])

# Statistics — 20x over Python's statistics module
stats = batch_stats([1.0, 2.0, 3.0, 4.0, 5.0])
```

## Supported Django Fields

| Field | Rust Type | Notes |
|---|---|---|
| CharField, TextField, EmailField, URLField, SlugField | `String` | Character counting, not byte counting |
| IntegerField, BigIntegerField | `i64` | Full 64-bit range |
| DecimalField | `rust_decimal` | Full precision, never floats |
| DateField, DateTimeField, TimeField | `chrono` | ISO 8601 / RFC 3339 |
| UUIDField | `uuid` | Hyphenated string |
| BooleanField | `bool` | True/False, never 1/0 |
| FloatField | `f64` | NaN/Infinity rejected |
| JSONField | `serde_json` | Nested structures preserved |
| BinaryField | `Vec<u8>` | Base64 encoded |

## Requirements

- Python 3.11+ (pre-built wheels, no Rust installation needed)
- Python 3.14t supported (free-threading / no-GIL)
- Django 4.2 LTS or 5.x (for clarax-django)
- Any Python project (for clarax-core)

## License

MIT -- [Abdulwahed Mansour](https://github.com/abdulwahed-sweden)
