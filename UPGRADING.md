# Upgrading PyForge

## From v0.2.x to v0.3.0

**Breaking changes:** None. All existing APIs work identically.

**New features available immediately:**

- `serialize_values_list()` — fastest path for bulk serialization
- `pyforge_doctor` — find which serializers benefit from PyForge
- `from_dataclass()` — auto-generate Schema from dataclasses
- `PyForgeMetricsMiddleware` — per-request performance metrics
- `serialize_stream()` — constant-memory exports

**Migration checklist:**

```
[ ] pip install --upgrade pyforge-django
[ ] python manage.py pyforge_doctor
[ ] (Optional) Add PyForgeMetricsMiddleware to MIDDLEWARE
[ ] (Optional) Switch bulk endpoints to serialize_values_list()
[ ] (Optional) Add PYFORGE_METRICS = True to settings for observability
```

**For pyforge-core users:**

```
[ ] pip install --upgrade pyforge-core
[ ] Try from_dataclass() instead of manual Schema({...})
[ ] Field(int, max_length=100) now raises SchemaError (was silently ignored)
```
