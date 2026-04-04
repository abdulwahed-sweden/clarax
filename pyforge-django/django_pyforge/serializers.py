# Author: Abdulwahed Mansour
"""
Drop-in mixin for Django REST Framework serializers.

Accelerates the serialization phase using Rust-native field processing.
Works with existing DRF ModelSerializer — just add the mixin.

Usage:
    from django_pyforge.serializers import RustSerializerMixin
    from rest_framework import serializers

    class UserSerializer(RustSerializerMixin, serializers.ModelSerializer):
        class Meta:
            model = User
            fields = '__all__'

The mixin automatically detects which fields are simple model fields
(accelerated by Rust) vs computed fields (delegated to DRF).
"""

import logging

from django_pyforge import ModelSchema, serialize_instance

logger = logging.getLogger("django_pyforge")

# Sentinel for uninitialized schema cache
_UNINITIALIZED = object()

# Class-level cache for field classification results.
# Keyed by serializer class id to avoid recomputing per instance.
_field_cache = {}


class _PyForgeListSerializer:
    """
    Fast-path list serializer that bypasses DRF's ListSerializer entirely.

    Instead of calling to_representation() per instance through DRF's
    field pipeline, this calls serialize_instance() directly in a tight
    Python loop and patches in Python-delegated fields afterward.
    """

    def __init__(self, child_class, instances, rust_fields, python_field_names,
                 schema, ordered_field_names):
        self.child_class = child_class
        self.instances = instances
        self.rust_fields = rust_fields
        self.python_field_names = python_field_names
        self.schema = schema
        self.ordered_field_names = ordered_field_names

    @property
    def data(self):
        results = []
        schema = self.schema
        rust_fields = self.rust_fields
        python_field_names = self.python_field_names
        ordered = self.ordered_field_names
        has_python = bool(python_field_names)

        for instance in self.instances:
            try:
                row = serialize_instance(instance, schema)
            except Exception:
                # Fallback: delegate this instance to DRF
                child = self.child_class(instance)
                results.append(child.data)
                continue

            if has_python:
                # Patch in Python-delegated fields
                for field_name in python_field_names:
                    if field_name == "id":
                        row["id"] = instance.pk
                    elif hasattr(instance, field_name + "_id"):
                        # FK field — use the _id attribute directly
                        row[field_name] = getattr(instance, field_name + "_id")
                    else:
                        row[field_name] = getattr(instance, field_name, None)

            # Reorder to match serializer field declaration order
            if ordered:
                row = {k: row[k] for k in ordered if k in row}

            results.append(row)

        return results


class RustSerializerMixin:
    """
    Mixin that accelerates DRF serializer .to_representation() using Rust.

    Add this mixin BEFORE the base serializer class in the MRO:

        class MySerializer(RustSerializerMixin, serializers.ModelSerializer):
            ...

    Behavior:
    - On first use, compiles a ModelSchema from the model class and caches it.
    - For single instances, extracts simple model fields via Rust (single call).
    - For many=True, bypasses DRF's ListSerializer entirely when 80%+ of
      fields are Rust-supported — calls serialize_instance() in a tight loop.
    - Delegates computed fields (SerializerMethodField, source overrides,
      nested serializers) to Python.
    - Falls back to DRF silently on any error.
    """

    _pyforge_schema = _UNINITIALIZED

    @classmethod
    def _init_pyforge_schema(cls):
        """Build the schema on first use. Cached on the class."""
        if cls._pyforge_schema is not _UNINITIALIZED:
            return

        try:
            model_class = cls.Meta.model
        except AttributeError:
            cls._pyforge_schema = None
            return

        try:
            cls._pyforge_schema = ModelSchema(model_class)
        except Exception as exc:
            logger.debug("PyForge: could not compile schema for %s: %s",
                         model_class.__name__, exc)
            cls._pyforge_schema = None

    @classmethod
    def _get_field_classification(cls, serializer_instance):
        """
        Classify fields into Rust-accelerated and Python-delegated sets.
        Result is cached per class — never recalculated for the same serializer class.
        """
        cache_key = id(cls)
        cached = _field_cache.get(cache_key)
        if cached is not None:
            return cached

        cls._init_pyforge_schema()

        if cls._pyforge_schema is None:
            result = (set(), set(serializer_instance.fields.keys()),
                      list(serializer_instance.fields.keys()))
            _field_cache[cache_key] = result
            return result

        rust_schema_fields = set(cls._pyforge_schema.field_names_list)
        rust_fields = set()
        python_fields = set()

        for field_name, field_obj in serializer_instance.fields.items():
            field_class_name = type(field_obj).__name__

            is_computed = field_class_name in (
                "SerializerMethodField",
                "HiddenField",
                "ReadOnlyField",
            )
            is_nested = hasattr(field_obj, "Meta") and hasattr(field_obj, "fields")
            source = getattr(field_obj, "source", field_name)
            has_custom_source = source != field_name and source != "*"
            in_rust_schema = field_name in rust_schema_fields

            if in_rust_schema and not is_computed and not is_nested and not has_custom_source:
                rust_fields.add(field_name)
            else:
                python_fields.add(field_name)

        ordered = list(serializer_instance.fields.keys())
        result = (rust_fields, python_fields, ordered)
        _field_cache[cache_key] = result
        return result

    def __init_subclass__(cls, **kwargs):
        """Clear cache when a new subclass is defined."""
        super().__init_subclass__(**kwargs)
        _field_cache.pop(id(cls), None)

    def __class_getitem__(cls, params):
        """Support type hints without breaking cache."""
        return cls

    @classmethod
    def many_init(cls, *args, **kwargs):
        """
        Override DRF's many_init to use the fast-path list serializer
        when conditions are met (80%+ Rust-supported fields).
        """
        instances = args[0] if args else kwargs.get("instance")
        many = kwargs.pop("many", False)

        if not many or instances is None:
            # Not a many=True call — use default DRF path
            return super(RustSerializerMixin, cls).many_init(*args, **kwargs)

        # We need a temporary instance to classify fields
        cls._init_pyforge_schema()
        if cls._pyforge_schema is None:
            kwargs["many"] = True
            return super(RustSerializerMixin, cls).many_init(*args, **kwargs)

        # Create a throwaway serializer to inspect fields
        try:
            temp = cls()
            rust_fields, python_fields, ordered = cls._get_field_classification(temp)
        except Exception:
            kwargs["many"] = True
            return super(RustSerializerMixin, cls).many_init(*args, **kwargs)

        total = len(rust_fields) + len(python_fields)
        rust_ratio = len(rust_fields) / total if total > 0 else 0

        # Only use fast path if 80%+ of fields are Rust-supported
        if rust_ratio >= 0.80:
            try:
                return _PyForgeListSerializer(
                    child_class=cls,
                    instances=instances,
                    rust_fields=rust_fields,
                    python_field_names=python_fields,
                    schema=cls._pyforge_schema,
                    ordered_field_names=ordered,
                )
            except Exception:
                pass

        # Fall back to DRF's ListSerializer
        kwargs["many"] = True
        return super(RustSerializerMixin, cls).many_init(*args, **kwargs)

    def to_representation(self, instance):
        """
        Serialize a single model instance using Rust for simple fields.
        Uses cached field classification — no per-instance overhead.
        """
        cls = type(self)
        cls._init_pyforge_schema()

        if cls._pyforge_schema is None:
            return super().to_representation(instance)

        rust_fields, python_fields, ordered = cls._get_field_classification(self)

        if not rust_fields:
            return super().to_representation(instance)

        try:
            rust_result = serialize_instance(instance, cls._pyforge_schema)

            if not python_fields:
                return {k: rust_result[k] for k in ordered if k in rust_result}

            result = {}
            for field_name in rust_fields:
                if field_name in rust_result:
                    result[field_name] = rust_result[field_name]

            for field_name in python_fields:
                if field_name == "id":
                    result[field_name] = instance.pk
                elif hasattr(instance, field_name + "_id"):
                    result[field_name] = getattr(instance, field_name + "_id")
                else:
                    field_obj = self.fields[field_name]
                    try:
                        attr = field_obj.get_attribute(instance)
                        result[field_name] = field_obj.to_representation(attr)
                    except Exception:
                        result[field_name] = None

            return {k: result[k] for k in ordered if k in result}

        except Exception as exc:
            logger.debug("PyForge: Rust serialization failed, falling back to DRF: %s", exc)
            return super().to_representation(instance)
