[![PyPI django](https://img.shields.io/pypi/v/pyforge-django.svg)](https://pypi.org/project/pyforge-django)
[![PyPI core](https://img.shields.io/pypi/v/pyforge-core.svg)](https://pypi.org/project/pyforge-core)
[![crates.io](https://img.shields.io/crates/v/pyforge-core.svg)](https://crates.io/crates/pyforge-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Python 3.11+](https://img.shields.io/badge/python-3.11%2B-blue.svg)](https://www.python.org)
[![Django 4.2+](https://img.shields.io/badge/django-4.2%2B-green.svg)](https://www.djangoproject.com)

# PyForge

Rust-accelerated serialization and validation for Python. Drop-in for Django REST Framework, standalone for everything else.

## Performance

Measured against DRF `ModelSerializer` on a 9-field model (CPython 3.12, Django 6.0):

| Scenario | DRF | PyForge | Speedup |
|---|---|---|---|
| Serialize 100 instances | 40.8 ms | 1.2 ms | **33x** |
| Serialize 1,000 instances | 475 ms | 14.6 ms | **33x** |
| Serialize 3,000 via `values_list()` | 166 ms | 47.6 ms | **3.5x** |
| Validate 1,000 instances | 506 ms | 10.2 ms | **50x** |

Database query time excluded. Raw dict comprehensions show no benefit — PyForge replaces DRF, not Python itself.

## Install

```bash
pip install pyforge-django        # Django projects
pip install pyforge-core           # Any Python project
```

Add to `INSTALLED_APPS`: `"django_pyforge"`

## Django Quickstart

```python
# Add to any existing DRF serializer — one line change
from django_pyforge.serializers import RustSerializerMixin

class MySerializer(RustSerializerMixin, serializers.ModelSerializer):
    class Meta:
        model = MyModel
        fields = "__all__"

# Check which serializers benefit:
# python manage.py pyforge_doctor
```

## Python Quickstart

```python
from dataclasses import dataclass
from pyforge_core import from_dataclass, serialize

@dataclass
class User:
    name: str
    age: int
    email: str

schema = from_dataclass(User)
result = serialize({"name": "Erik", "age": 30, "email": "erik@x.com"}, schema)
```

## What's New in v0.3.0

- `serialize_values_list()` — single Rust call for entire querysets (3.5x over DRF)
- `pyforge_doctor` — audit which serializers benefit (`--app`, `--json`, `--threshold`)
- `from_dataclass()` / `from_typeddict()` — auto-generate Schema from type hints
- `PyForgeMetricsMiddleware` — `X-PyForge-Stats` header per request
- N+1 detection — warns on missing `select_related` during serialization
- `serialize_stream()` — constant-memory streaming for large exports
- Schema validation — catches invalid `Field()` constraints at definition time

## Supported Django Fields

| Field | Notes |
|---|---|
| CharField, TextField, EmailField, URLField, SlugField | Character counting, not byte counting |
| IntegerField, BigIntegerField | i64 range |
| DecimalField | Full precision via `rust_decimal` — never floats |
| DateField, DateTimeField, TimeField | ISO 8601 / RFC 3339 |
| UUIDField | Hyphenated string |
| BooleanField | True/False, never 1/0 |
| FloatField | NaN/Infinity rejected |
| JSONField | Nested structures preserved |
| BinaryField | Base64 encoded |

## When PyForge Helps

- DRF list views returning 50+ records
- Bulk create/update with validation
- Export jobs processing thousands of records
- High-traffic APIs where serialization is the bottleneck

## When PyForge Does NOT Help

- Raw dict comprehensions or `.values()` without DRF
- Single-record detail views (bridge overhead ~10us)
- Database-bound views — PyForge does not touch query time

## Requirements

- Python 3.11+ — no Rust installation needed (pre-built wheels)
- Django 4.2 LTS or 5.x (for pyforge-django)
- Any Python project (for pyforge-core)

## License

MIT — [Abdulwahed Mansour](https://github.com/abdulwahed-sweden)
