# pyforge-core

Rust-accelerated serialization and validation for any Python project. No framework required.

## Install

```bash
pip install pyforge-core
```

## Quickstart

```python
from dataclasses import dataclass
from pyforge_core import from_dataclass, serialize, validate

@dataclass
class User:
    name: str
    age: int
    email: str

schema = from_dataclass(User)
result = serialize({"name": "Erik", "age": 30, "email": "erik@x.com"}, schema)
report = validate({"name": "Erik", "age": -5, "email": "erik@x.com"}, schema)
```

## Manual Schema

```python
from pyforge_core import Schema, Field
from decimal import Decimal

schema = Schema({
    "name": Field(str, max_length=100),
    "price": Field(Decimal, max_digits=10, decimal_places=2),
    "active": Field(bool),
})
```

## Supported Types

| Type | Constraints |
|---|---|
| `str` | `max_length`, `min_length` |
| `int` | `min_value`, `max_value` |
| `float` | `min_value`, `max_value` |
| `bool` | — |
| `Decimal` | `max_digits`, `decimal_places` |
| `datetime` | — |
| `date` | — |
| `time` | — |
| `UUID` | — |
| `list` | — |
| `dict` | — |
| `bytes` | `max_length` |

All types accept `nullable=True` and `default=True`.

Invalid constraints raise `SchemaError` immediately: `Field(int, max_length=100)` fails at definition time.

## When to Use

Flask, FastAPI, scripts, ETL pipelines, data validation — any Python code that processes structured data.

For Django projects, use [`pyforge-django`](https://pypi.org/project/pyforge-django/) which adds `ModelSchema`, `RustSerializerMixin`, and automatic Django model introspection on top of pyforge-core.

## License

MIT — Abdulwahed Mansour
