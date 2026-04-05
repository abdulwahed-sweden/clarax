# Author: Abdulwahed Mansour
"""
Audit all ModelSerializer classes and report PyForge compatibility.

Usage:
    python manage.py pyforge_doctor
    python manage.py pyforge_doctor --app queue
    python manage.py pyforge_doctor --threshold 0.7
    python manage.py pyforge_doctor --json
"""

import json as json_mod

from django.core.management.base import BaseCommand


class Command(BaseCommand):
    help = (
        "Audit all ModelSerializer classes in the project and report "
        "which ones benefit from PyForge acceleration."
    )

    def add_arguments(self, parser):
        parser.add_argument(
            "--app",
            type=str,
            default=None,
            help="Audit only serializers from a specific Django app.",
        )
        parser.add_argument(
            "--threshold",
            type=float,
            default=0.8,
            help="Minimum Rust field ratio to recommend (default: 0.8).",
        )
        parser.add_argument(
            "--json",
            action="store_true",
            default=False,
            help="Output results as JSON for CI integration.",
        )

    def handle(self, *args, **options):
        try:
            from rest_framework.serializers import ModelSerializer
        except ImportError:
            self.stderr.write("djangorestframework is not installed.")
            return

        from django_pyforge import ModelSchema

        app_filter = options["app"]
        threshold = options["threshold"]
        output_json = options["json"]

        serializers = self._discover_serializers(ModelSerializer, app_filter)

        if not serializers:
            if output_json:
                self.stdout.write(json_mod.dumps({"serializers": [], "summary": {"recommended": 0, "skipped": 0}}))
            else:
                self.stdout.write("No ModelSerializer subclasses found.")
            return

        results = []
        for ser_class in serializers:
            result = self._analyze_serializer(ser_class, ModelSchema, threshold)
            result["class"] = ser_class.__qualname__
            result["module"] = ser_class.__module__
            results.append(result)

        if output_json:
            recommended = sum(1 for r in results if r["recommendation"] == "RECOMMENDED")
            skipped = sum(1 for r in results if r["recommendation"] == "SKIP")
            self.stdout.write(json_mod.dumps({
                "serializers": results,
                "summary": {"recommended": recommended, "skipped": skipped, "threshold": threshold},
            }, indent=2))
            return

        self.stdout.write("")
        self.stdout.write("PyForge Serializer Audit")
        self.stdout.write("=" * 60)
        self.stdout.write("")

        for result in results:
            name = result["class"]
            if result["recommendation"] == "RECOMMENDED":
                tag = self.style.SUCCESS("RECOMMENDED")
            elif result["recommendation"] == "MARGINAL":
                tag = self.style.WARNING("MARGINAL")
            else:
                tag = self.style.ERROR("SKIP")

            self.stdout.write(
                f"  {name:<35} "
                f"{result['rust_fields']:>2}/{result['total_fields']:<2} "
                f"Rust fields ({result['ratio']:>3.0%}) -> {tag}"
            )
            for reason in result.get("python_reasons", [])[:3]:
                self.stdout.write(f"    - {reason}")

        recommended = sum(1 for r in results if r["recommendation"] == "RECOMMENDED")
        skipped = sum(1 for r in results if r["recommendation"] == "SKIP")
        self.stdout.write("")
        self.stdout.write("-" * 60)
        self.stdout.write(f"  Recommended: {recommended}  |  Skipped: {skipped}  |  Threshold: {threshold:.0%}")
        self.stdout.write("")

    def _discover_serializers(self, base_class, app_filter=None):
        found = []
        seen = set()
        queue = list(base_class.__subclasses__())

        while queue:
            cls = queue.pop(0)
            if id(cls) in seen:
                continue
            seen.add(id(cls))

            if cls.__module__.startswith("rest_framework"):
                queue.extend(cls.__subclasses__())
                continue

            if app_filter and not cls.__module__.startswith(f"apps.{app_filter}"):
                queue.extend(cls.__subclasses__())
                continue

            if hasattr(cls, "Meta") and hasattr(cls.Meta, "model"):
                found.append(cls)

            queue.extend(cls.__subclasses__())

        return found

    def _analyze_serializer(self, ser_class, model_schema_cls, threshold):
        try:
            model = ser_class.Meta.model
            schema = model_schema_cls(model)
            rust_field_names = set(schema.field_names_list)
        except Exception:
            return {
                "rust_fields": 0, "total_fields": 0, "ratio": 0,
                "recommendation": "SKIP",
                "python_reasons": ["Could not compile ModelSchema"],
            }

        try:
            ser = ser_class()
            fields = ser.fields
        except Exception:
            return {
                "rust_fields": 0, "total_fields": 0, "ratio": 0,
                "recommendation": "SKIP",
                "python_reasons": ["Could not instantiate serializer"],
            }

        rust_count = 0
        python_reasons = []

        for field_name, field_obj in fields.items():
            field_class_name = type(field_obj).__name__
            is_computed = field_class_name in ("SerializerMethodField", "HiddenField", "ReadOnlyField")
            is_nested = hasattr(field_obj, "Meta") and hasattr(field_obj, "fields")
            source = getattr(field_obj, "source", field_name)
            has_custom_source = source != field_name and source != "*"
            in_schema = field_name in rust_field_names

            if in_schema and not is_computed and not is_nested and not has_custom_source:
                rust_count += 1
            else:
                reason = field_name
                if is_computed:
                    reason += f" ({field_class_name})"
                elif is_nested:
                    reason += " (nested serializer)"
                elif has_custom_source:
                    reason += f" (custom source='{source}')"
                elif not in_schema:
                    reason += " (not a model field)"
                python_reasons.append(reason)

        total = len(fields)
        ratio = rust_count / total if total > 0 else 0

        if ratio >= threshold and total >= 5:
            rec = "RECOMMENDED"
        elif ratio >= threshold * 0.75:
            rec = "MARGINAL"
        else:
            rec = "SKIP"

        return {
            "rust_fields": rust_count, "total_fields": total, "ratio": ratio,
            "recommendation": rec, "python_reasons": python_reasons,
        }
