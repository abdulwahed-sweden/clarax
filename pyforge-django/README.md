# pyforge-django

Rust-accelerated serialization and validation for Django REST Framework.

## Installation

```bash
pip install pyforge-django
```

## Quick Start

```python
from django_pyforge import ModelSchema, serialize_instance
from django_pyforge.serializers import RustSerializerMixin
from rest_framework import serializers

# Compile schema once at startup
schema = ModelSchema(MyModel)

# Serialize — one Rust call, all fields
result = serialize_instance(instance, schema)

# Or use the DRF mixin — zero code changes
class MySerializer(RustSerializerMixin, serializers.ModelSerializer):
    class Meta:
        model = MyModel
        fields = "__all__"
```

## Performance

3-8x faster than pure Python DRF for 100+ record batches.

See [BENCHMARKS.md](https://github.com/abdulwahed-sweden/pyforge/blob/main/BENCHMARKS.md) for full results.

## Requirements

- Python 3.11+
- Django 4.2 LTS or 5.x

## License

MIT — Abdulwahed Mansour
