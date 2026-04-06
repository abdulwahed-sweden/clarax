# Upgrading ClaraX

## From v0.3.x to v1.0.0

**Breaking changes:** None. All existing APIs work identically.

**What changed internally:**

- `serialize_many`: uses `PyDict_Copy` + selective overwrite (2.2x over pure Python)
- `validate_many`: inline validation without `FieldValue` intermediary (1.6x over pure Python)
- New batch functions: `validate_names_batch`, `validate_ids_batch`, `compute_risk_batch`, `batch_stats`
- Python 3.14t (free-threading) support — zero code changes required

**Migration checklist:**

```
[ ] pip install --upgrade clarax-django clarax-core
[ ] python manage.py clarax_doctor
[ ] (Optional) Use batch functions for heavy validation workloads
```

## From v0.2.x to v0.3.0

**Breaking changes:** None. All existing APIs work identically.

**New features:**

- `serialize_values_list()` — fastest path for bulk serialization
- `clarax_doctor` — find which serializers benefit from ClaraX
- `from_dataclass()` — auto-generate Schema from dataclasses
- `ClaraXMetricsMiddleware` — per-request performance metrics
- `serialize_stream()` — constant-memory exports
- Schema validation — catches invalid `Field()` constraints at definition time

**Migration checklist:**

```
[ ] pip install --upgrade clarax-django
[ ] python manage.py clarax_doctor
[ ] (Optional) Add ClaraXMetricsMiddleware to MIDDLEWARE
[ ] (Optional) Switch bulk endpoints to serialize_values_list()
```

**For clarax-core users:**

```
[ ] pip install --upgrade clarax-core
[ ] Try from_dataclass() instead of manual Schema({...})
[ ] Field(int, max_length=100) now raises SchemaError (was silently ignored)
```
