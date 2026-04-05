# pyforge-django

Rust-accelerated serialization and validation for Django REST Framework. 33x faster than `ModelSerializer`.

## Install

```bash
pip install pyforge-django
```

Add `"django_pyforge"` to `INSTALLED_APPS`.

## Quick Start

```python
from django_pyforge.serializers import RustSerializerMixin
from rest_framework import serializers

class MySerializer(RustSerializerMixin, serializers.ModelSerializer):
    class Meta:
        model = MyModel
        fields = "__all__"
```

Check which serializers benefit: `python manage.py pyforge_doctor`

## Performance

| Scenario | DRF | PyForge | Speedup |
|---|---|---|---|
| Serialize 1,000 instances | 475 ms | 14.6 ms | **33x** |
| Serialize 3,000 via `values_list()` | 166 ms | 47.6 ms | **3.5x** |
| Validate 1,000 instances | 506 ms | 10.2 ms | **50x** |

## v0.3.0 Features

- `serialize_values_list()` — single Rust call for entire querysets
- `pyforge_doctor` — audit serializers (`--app`, `--json`, `--threshold`)
- `PyForgeMetricsMiddleware` — `X-PyForge-Stats` per request
- N+1 detection — warns on missing `select_related`
- `serialize_stream()` — constant-memory exports

## Requirements

- Python 3.11+
- Django 4.2 LTS or 5.x

## License

MIT — Abdulwahed Mansour
