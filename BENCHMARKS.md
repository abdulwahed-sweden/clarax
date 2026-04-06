# ClaraX Benchmarks

**Author:** Abdulwahed Mansour

## Django (clarax-django vs DRF)

### Methodology

- **Baseline:** DRF `ModelSerializer` with `fields = "__all__"`
- **Model:** 17-field model (CharField, IntegerField, DecimalField, DateField, DateTimeField, UUIDField, BooleanField, FloatField)
- **Measurement:** `statistics.median()` over 5+ runs
- **Database:** in-memory SQLite (removes query time from results)
- **Environment:** Python 3.12, Django 6.0, DRF 3.17, ClaraX 1.0.1, macOS x86_64

### Results

| Benchmark | DRF | ClaraX | Speedup |
|---|---|---|---|
| Serialize 100 instances | 40.8 ms | 1.2 ms | **33x** |
| Serialize 1,000 instances | 475 ms | 14.6 ms | **33x** |
| Serialize 3,000 via `values_list()` | 166 ms | 47.6 ms | **3.5x** |
| Validate 1,000 instances | 506 ms | 10.2 ms | **50x** |

### Real project (Hyra, 17-field QueueEntry model)

| Method | 500 records | vs DRF |
|---|---|---|
| Pure DRF ModelSerializer | 57 ms | baseline |
| DRF + `RustSerializerMixin` | 26 ms | **2.2x** |
| `serialize_batch()` | 21 ms | **2.7x** |

### What the numbers mean

**33x on serialization:** DRF resolves field descriptors, runs type coercion, and dispatches method calls per field per instance. ClaraX compiles the schema once at startup and runs a single Rust call per batch.

**50x on validation:** DRF runs the full validator chain per field per instance. ClaraX runs parallel validation across the entire batch in one Rust call.

**2.2-2.7x on real projects:** Real serializers have Python-delegated fields (ForeignKey, SerializerMethodField) that ClaraX can't accelerate. The mixin accelerates the Rust-compatible fields and delegates the rest.

## Standalone (clarax-core, dict workloads)

### Batch operations (50K records)

| Operation | Python | ClaraX | Speedup |
|---|---|---|---|
| Name validation (150K strings) | 2,147 ms | 237 ms | **9.1x** |
| Pattern matching (50K IDs) | 46 ms | 3 ms | **15.5x** |
| Risk computation (50K records) | 238 ms | 64 ms | **3.7x** |
| Batch statistics (50K values) | 96 ms | 5 ms | **20.8x** |
| Full validation combined | 2,835 ms | 495 ms | **5.7x** |

### Schema serialization (50K x 11 fields)

| Benchmark | Python | ClaraX | Speedup |
|---|---|---|---|
| Serialize 1,000 dicts | 2.1 ms | 0.7 ms | **3.0x** |
| Serialize 50,000 dicts | 284 ms | 127 ms | **2.2x** |
| Validate 50,000 dicts | 200 ms | 128 ms | **1.6x** |

### Why batch operations are faster than schema operations

Batch operations (`validate_names_batch`, `validate_ids_batch`, `batch_stats`) extract data once, then process entirely in Rust with Rayon parallelism. The computation dominates.

Schema operations (`serialize_many`, `validate_many`) access Python dicts field by field through CPython's C API. Both ClaraX and pure Python use the same dict operations. ClaraX eliminates Python bytecode overhead but cannot bypass the dict API itself. The ceiling is ~3x.

### Python 3.14 free-threading

ClaraX supports Python 3.14t (no-GIL) with zero code changes:

| Operation | 3.12 (GIL) | 3.14t (no-GIL) | Change |
|---|---|---|---|
| serialize_many 50K | 54 ms | 47 ms | **13% faster** |
| validate_names 150K | 106 ms | 77 ms | **27% faster** |

Operations with heavy Rayon-parallel computation benefit. Operations dominated by Python object access see less gain (per-object locks replace the GIL).

## When ClaraX helps

- DRF list views returning 50+ records
- Bulk create/update with validation
- Export jobs processing thousands of records
- Batch string validation (character scanning, pattern matching)
- High-traffic APIs where serialization is the bottleneck

## When ClaraX does NOT help

- Single-record detail endpoints (~10us bridge overhead)
- Database-bound views (optimize queries first)
- Views where every field is `SerializerMethodField`
- Simple dict processing (Python is already near C-speed)

## Per-field cost (Rust micro-benchmarks)

| Django Field | Rust Type | Time (median) |
|---|---|---|
| BooleanField | `bool` | 252 ns |
| IntegerField | `i64` | 260 ns |
| CharField | `String` | 391 ns |
| UUIDField | `uuid::Uuid` | 416 ns |
| DecimalField | `rust_decimal::Decimal` | 471 ns |
| DateTimeField | `chrono::DateTime` | 485 ns |
| DateField | `chrono::NaiveDate` | 677 ns |
| JSONField | `serde_json::Value` | 1.94 us |

## Reproducing

```bash
pip install "django>=5.0" djangorestframework clarax-django
python manage.py clarax_doctor   # check your project
```

Rust micro-benchmarks (requires source checkout):

```bash
cargo bench -p clarax-django
```
