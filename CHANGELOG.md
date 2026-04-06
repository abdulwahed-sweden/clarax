# Changelog

All notable changes to ClaraX will be documented here.

Format: [keepachangelog.com](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-04-06

### Changed
- **Version bump to 1.0.0** — ClaraX is production-ready
- Development Status upgraded from Alpha to Production/Stable

### Added
- **Python 3.14 free-threading support** — ClaraX wheels build and run on `python3.14t` (no-GIL) with zero code changes. Rayon parallel operations use all CPU cores with zero GIL contention.
- **Batch validation functions** for workloads where Rust dominates Python:
  - `validate_names_batch()` — parallel character scanning (9x over Python)
  - `validate_ids_batch()` — zero-copy pattern matching (15x over Python)
  - `compute_risk_batch()` — parallel float math with Rayon (3.7x over Python)
  - `batch_stats()` — parallel statistics: mean, median, stdev (20x over Python)
- **"When to Use ClaraX" guide** in README — clear guidance on which Django workloads benefit

### Performance (clarax-core, 50K records)
- `serialize_many`: 2.2x over pure Python (dict.copy + selective overwrite)
- `validate_names_batch`: 9.1x over pure Python (Rayon parallel char scanning)
- `validate_ids_batch`: 15.5x over pure Python (zero-copy pattern matching)
- `batch_stats`: 20.8x over pure Python (parallel reduce + sort)
- Full application validation: 5.7x combined speedup

### Performance (clarax-django, QueueEntry x500)
- `RustSerializerMixin`: 2.2x over pure DRF ModelSerializer
- `serialize_batch()`: 2.7x over pure DRF ModelSerializer

## [0.3.1] — 2026-04-06

### Fixed
- **clarax-core**: `serialize_many` and `validate_many` now faster than pure Python on dict-based workloads of 1,000+ records
  - Serialization: `PyDict_Copy` shallow-copies entire input dict, only overwrites Decimal/UUID/datetime fields — passthrough fields (str, int, float, bool) never individually accessed
  - Validation: inline validation from Python objects — string length via `len()` (O(1)), Decimal digits via zero-copy `to_str()` string inspection (no `rust_decimal` parsing), bool by isinstance only
  - Pre-interned field names, pre-classified field types at Schema construction time
  - Eliminated 550K descriptor clones per 50K-record batch

### Performance
- `serialize_many` 50K dicts: **2.2x** faster than pure Python (was 0.3x — 7.3x improvement)
- `serialize_many` 1K dicts: **3.0x** faster than pure Python
- `validate_many` 50K dicts: **1.6x** faster than pure Python (was 0.3x — 5.3x improvement)
- `validate_many` 1K dicts: **1.0x** parity (was 0.03x due to Rayon thread pool startup)

## [0.3.0] — 2026-04-05

### Added
- `serialize_values_list()` — accepts `QuerySet.values_list()` output directly, single Python-Rust bridge crossing for the entire batch
- `clarax_doctor` management command — audits all ModelSerializer classes, reports Rust field ratio and recommendation. Supports `--app`, `--threshold`, `--json` flags
- `from_dataclass()` and `from_typeddict()` — auto-generate Schema from Python type annotations, supports `Optional`, `Annotated` with constraint markers
- `ClaraXMetricsMiddleware` — adds `X-ClaraX-Stats` response header with rust_fields, python_fields, calls, and milliseconds per request
- N+1 query detection — warns when ForeignKey fields are not in `select_related` during serialization (DEBUG mode only)
- `serialize_stream()` — yields serialized chunks from a queryset for `StreamingHttpResponse`, constant memory regardless of queryset size
- Compile-time schema validation — `Field(int, max_length=100)` raises `SchemaError` at definition time instead of silently ignoring the constraint
- Benchmark regression CI — GitHub Actions workflow runs benchmarks on PRs touching clarax-core or clarax-django
- Constraint markers for `Annotated`: `MaxLength`, `MinLength`, `MinValue`, `MaxValue`, `MaxDigits`, `DecimalPlaces`

### Performance
- `serialize_values_list`: 3.5x over DRF for 3,000 records (up from 2.5x with mixin)
- `RustSerializerMixin` many=True: 2.2x over DRF (cached field classification + DRF bypass)
- Validate 1,000 instances: 50x over DRF

## [0.2.0] — 2026-04-05

### Added
- **clarax-core**: framework-agnostic Rust serialization and validation engine
  - `Schema`, `Field`, `serialize`, `serialize_many`, `validate`, `validate_many`
  - 12 supported types: str, int, float, bool, Decimal, datetime, date, time, UUID, list, dict, bytes
  - Rayon parallel validation at 128-entry threshold
  - 34 Rust unit tests
- Python package `clarax-core` on PyPI — works without Django

### Changed
- **clarax-django** now delegates all serialization and validation to clarax-core
  - 750 lines of duplicated logic removed
  - 150 lines of thin delegation wrappers added
  - `DjangoFieldType.to_core_type()` maps Django's 16 field types to clarax-core's 12 types
- `RustSerializerMixin` rewritten: cached field classification + DRF bypass on `many=True`
  - 2.3-3.2x speedup over DRF (up from 1.4-1.5x in v0.1.x)
- All crate versions bumped to 0.2.0

### Fixed
- Benchmark methodology: now measured against DRF ModelSerializer (real baseline),
  not raw dict comprehensions

### Stats
- 1,350 tests passing across the workspace
- Zero clippy warnings
- Zero code duplication between clarax-core and clarax-django

## [0.1.0] — 2026-04-04

### Added

- `ModelSchema`: compiled field descriptor cache built once at Django startup,
  reused on every request with zero per-request parsing overhead
- `serialize_instance()`: single-call serialization — extracts all field values
  from a Django model instance in one Rust call, reducing boundary crossings from N+1 to 1
- `serialize_batch()`: queryset serialization processing a list of instances in one call
- `validate_instance()`: schema-based validation with structured error output and `is_valid` flag
- `RustSerializerMixin`: drop-in DRF mixin that automatically classifies fields into
  Rust-accelerated (simple model fields) and Python-delegated (SerializerMethodField,
  nested serializers, custom source attributes)
- Rayon parallel validation for batches above 64 fields with deterministic error ordering
- Full Django field type support: CharField, TextField, IntegerField, BigIntegerField,
  DecimalField, DateField, TimeField, DateTimeField, UUIDField, BooleanField,
  FloatField, JSONField, BinaryField, EmailField, URLField, SlugField
- DecimalField precision preservation — values serialized as strings, never converted to float
- CharField max_length validation counts characters, not bytes (correct for multi-byte UTF-8)
- ASGI-compatible async bridge with proper GIL release for Django async views
- Django 4.2 LTS and 5.x support
- Python 3.11, 3.12, 3.13 support
- CPython-only build (PyPy and GraalPy removed)
- Minimum Python version raised to 3.11 (removed 3.8/3.9/3.10 compatibility shims)
- Criterion micro-benchmarks for serializer, validator, and per-field-type cost
- Python comparison benchmarks against pure Python DRF-style serialization
- PEP 561 type stubs for IDE autocompletion
- 33 Rust unit tests + 13 Django integration tests + 898 core tests = 944 total

### Security

- All string inputs validated through ClaraX extract() — no UTF-8 panic paths
- Memory exhaustion protection via Django's field size limit enforcement
- Bounds-checked indexing throughout — no buffer overflow vectors
- No `unwrap()` on user-controlled input — all fallible operations return `DjangoError`

[0.1.0]: https://github.com/abdulwahed-sweden/clarax/releases/tag/v0.1.0
