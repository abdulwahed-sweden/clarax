<p align="center">
  <img src="assets/logo-full.png" alt="ClaraX" width="420">
</p>

<p align="center">
  <a href="https://pypi.org/project/clarax-django"><img src="https://img.shields.io/pypi/v/clarax-django?label=clarax-django&color=blue" alt="clarax-django on PyPI"></a>
  <a href="https://pypi.org/project/clarax-core"><img src="https://img.shields.io/pypi/v/clarax-core?label=clarax-core&color=blue" alt="clarax-core on PyPI"></a>
  <a href="https://crates.io/crates/clarax-core"><img src="https://img.shields.io/crates/v/clarax-core?label=crates.io&color=orange" alt="clarax-core on crates.io"></a>
  <br>
  <a href="https://www.python.org"><img src="https://img.shields.io/badge/python-3.11%2B-3776AB?logo=python&logoColor=white" alt="Python 3.11+"></a>
  <a href="https://www.djangoproject.com"><img src="https://img.shields.io/badge/django-4.2%2B-092E20?logo=django&logoColor=white" alt="Django 4.2+"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT-green" alt="MIT License"></a>
  <a href="https://github.com/abdulwahed-sweden/clarax/releases"><img src="https://img.shields.io/github/v/release/abdulwahed-sweden/clarax?label=release&color=brightgreen" alt="GitHub Release"></a>
</p>

---

## Your Django project got slow. You know where.

It started fast. A few models, a few serializers, clean responses under 50ms.

Then it grew. The leaderboard endpoint went from 80ms to 3 seconds. The bulk export started timing out. Your monitoring showed serialization eating 80% of CPU on every request.

You scaled the server. The bill doubled. Speed improved 20%.

---

## You tried things

`select_related` helped. But not enough.

You looked at Django Ninja. Rewrote a few endpoints. Lost three weeks. The team still needed DRF for everything else.

You tried Celery for the heavy jobs. The APIs were still slow. You considered raw SQL. Faster queries, but now you're validating by hand.

Someone suggested Rust. You looked at PyO3 for a weekend. Your team writes Python. That's not changing.

You cached everything. Stale data. Invalidation bugs. More complexity than the problem you started with.

---

## The problem isn't your code

DRF processes every field in Python. One by one. For every record.

Your 17-field serializer on 500 records means 8,500 Python function calls just for serialization. Validators add another layer on top. The GIL means one CPU core, no matter how many you're paying for.

The slow part isn't your logic. It's the machinery around it.

---

## Two lines. Same serializer. Rust underneath.

Before:

```python
class ApplicationSerializer(serializers.ModelSerializer):
    class Meta:
        model = Application
        fields = "__all__"
```

After:

```python
from django_clarax.serializers import RustSerializerMixin

class ApplicationSerializer(RustSerializerMixin, serializers.ModelSerializer):
    class Meta:
        model = Application
        fields = "__all__"
```

Same views. Same URLs. Same tests. Same team.

---

## Real numbers

Measured against DRF `ModelSerializer`. Not cherry-picked. Not theoretical.

| What you had | Before | After | Speedup |
|---|---|---|---|
| Serialize 1,000 records | 475 ms | 14 ms | **33x** |
| Validate 1,000 records | 506 ms | 10 ms | **50x** |
| Name validation (150K strings) | 2,806 ms | 339 ms | **8x** |
| Pattern matching (50K IDs) | 82 ms | 4 ms | **15x** |

Honest footnote: simple dict operations show 2.2x only. Not everything benefits. Run `clarax_doctor` to see what your project actually gains.

---

## How it works

You manage the restaurant. You decide the menu, the service, the experience.

ClaraX is the industrial kitchen equipment you didn't have to build. It processes orders faster, but you still run the kitchen. Your staff still speaks Python. The equipment just happens to be built in Rust.

Under the hood: ClaraX compiles your schema once at startup. Every request runs serialization and validation in a single Rust call per batch. No per-field Python dispatch. No method resolution. On Python 3.14t, Rayon uses all your CPU cores.

---

## Is it for you?

**Yes:**

- Your Django project has 1,000+ users
- List endpoints returning 50+ records
- Bulk operations: imports, exports, reports
- Complex validation at scale
- You want Python 3.14 free-threading support today

**Not yet:**

- Fewer than 1,000 users. DRF is fast enough. Don't add what you don't need.
- Database-bound views. Run `select_related` first. ClaraX doesn't touch queries.
- Every field is `SerializerMethodField`. Python-computed fields bypass Rust.

---

## Not sure? Ask your project.

```bash
python manage.py clarax_doctor
```

Scans every serializer. Tells you which ones benefit, which fields accelerate, and which to skip.

No commitment. No risk. Just information.

---

## Get started (30 seconds)

```bash
pip install clarax-django
```

```python
# settings.py
INSTALLED_APPS = [
    ...
    "django_clarax",
]
```

```python
# your heaviest serializer
from django_clarax.serializers import RustSerializerMixin

class YourSerializer(RustSerializerMixin, serializers.ModelSerializer):
    class Meta:
        model = YourModel
        fields = "__all__"
```

No Rust installation. No Cargo. Pre-built wheels for macOS, Linux, and Windows.

---

## For heavy lifting

When you need more than the mixin:

```python
from django_clarax import ModelSchema, serialize_batch, serialize_values_list

schema = ModelSchema(MyModel)

# 2.7x over DRF
results = serialize_batch(queryset, schema)

# 3.5x over DRF — lowest overhead path
results = serialize_values_list(queryset, schema)

# Constant memory for large exports
for chunk in serialize_stream(queryset, schema, chunk_size=500):
    yield chunk
```

---

## Without Django

ClaraX works with any Python project. Flask, FastAPI, scripts, ETL pipelines.

```bash
pip install clarax-core
```

```python
from clarax_core import Schema, Field, serialize_many, validate_names_batch
from decimal import Decimal

schema = Schema({
    "name":  Field(str, max_length=100),
    "price": Field(Decimal, max_digits=10, decimal_places=2),
})

data = [{"name": "Erik", "price": Decimal("199.99")}]
result = serialize_many(data, schema)

# Where Rust dominates: character scanning, pattern matching
results = validate_names_batch(["Erik Andersson"] * 50000)  # 8x over Python
```

---

## Where this came from

ClaraX was born from real Django projects in Stockholm. A rental queue system needed to rank thousands of applicants in real time. Python wasn't fast enough. Rewriting wasn't an option. So we built the bridge -- a Rust engine that Django developers never have to touch. That bridge became ClaraX.

---

## Supported fields

| Django Field | Rust Type | Notes |
|---|---|---|
| CharField, TextField, EmailField, SlugField | `String` | Character counting, not bytes |
| IntegerField, BigIntegerField | `i64` | Full 64-bit range |
| DecimalField | `rust_decimal` | Full precision. Never floats. |
| DateField, DateTimeField, TimeField | `chrono` | ISO 8601 |
| UUIDField | `uuid` | Hyphenated string |
| BooleanField | `bool` | True/False only |
| FloatField | `f64` | NaN/Infinity rejected |
| JSONField | `serde_json` | Nested structures preserved |
| BinaryField | `Vec<u8>` | Base64 encoded |

Python 3.11+ supported. Python 3.14t (free-threading) supported.

---

MIT -- [Abdulwahed Mansour](https://github.com/abdulwahed-sweden)
