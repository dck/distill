"""DeepEval metric definitions for evaluating distillation quality."""

import logging
from collections.abc import Mapping

from deepeval.metrics import GEval, SummarizationMetric
from deepeval.test_case import LLMTestCase, LLMTestCaseParams

from judge import SonnetJudge

log = logging.getLogger(__name__)

WEIGHTS: dict[str, float] = {
    "Completeness": 0.35,
    "Structure Preservation": 0.25,
    "Coherence": 0.25,
    "Compression Quality": 0.15,
}

CONFIG_WEIGHT_KEYS = {
    "completeness": "Completeness",
    "structure": "Structure Preservation",
    "coherence": "Coherence",
    "compression_quality": "Compression Quality",
}


def create_metrics(judge: SonnetJudge) -> list:
    """Create and return the 4 evaluation metrics."""
    completeness = SummarizationMetric(model=judge, threshold=0.5, n=5)

    structure = GEval(
        name="Structure Preservation",
        criteria=(
            "Evaluate whether the distilled text maintains the original chapter's "
            "organizational structure, logical order of arguments, and progression "
            "from introduction to conclusion. The distilled version should feel like "
            "the same chapter, just shorter — not a reorganized summary."
        ),
        evaluation_params=[LLMTestCaseParams.INPUT, LLMTestCaseParams.ACTUAL_OUTPUT],
        model=judge,
    )

    coherence = GEval(
        name="Coherence",
        criteria=(
            "Evaluate the coherence and readability of the distilled text as a "
            "standalone piece. Sentences should follow logically from one another, "
            "there should be no dangling references to removed content, transitions "
            "should be smooth, and the text should read as a continuous narrative — "
            "not a collection of disconnected fragments."
        ),
        evaluation_params=[LLMTestCaseParams.ACTUAL_OUTPUT],
        model=judge,
    )

    compression_quality = GEval(
        name="Compression Quality",
        criteria=(
            "Evaluate whether the distillation made good compression decisions. "
            "The removed content should be genuinely low-value (filler phrases, "
            "motivational padding, redundant restatements, verbose introductions). "
            "The retained content should be high-value (key arguments, concrete "
            "examples with names and data, frameworks, actionable advice, definitions, "
            "cause-effect relationships). Penalize if important content was cut or if "
            "filler was retained."
        ),
        evaluation_params=[LLMTestCaseParams.INPUT, LLMTestCaseParams.ACTUAL_OUTPUT],
        model=judge,
    )

    return [completeness, structure, coherence, compression_quality]


def get_metric_weights(config: Mapping | None = None) -> dict[str, float]:
    """Return display-name weights, optionally overridden from config.toml."""
    if config is None:
        return WEIGHTS.copy()

    raw_weights = config.get("metrics_weights", {})
    weights = WEIGHTS.copy()
    for raw_key, display_name in CONFIG_WEIGHT_KEYS.items():
        value = raw_weights.get(raw_key)
        if value is not None:
            weights[display_name] = float(value)
    return weights


def compute_composite_score(
    scores: dict[str, float], weights: dict[str, float] | None = None
) -> float:
    """Compute weighted composite score."""
    if weights is None:
        weights = WEIGHTS
    total = sum(
        scores[key] * weights[key] for key in weights if scores.get(key) is not None
    )
    weight_sum = sum(
        weights[key] for key in weights if scores.get(key) is not None
    )
    return total / weight_sum if weight_sum > 0 else 0.0


def compute_compression_ratio(original: str, distilled: str) -> float:
    """Compute compression ratio: len(distilled) / len(original)."""
    if not original:
        return 0.0
    return len(distilled) / len(original)


def evaluate_chapter(
    original_text: str,
    distilled_text: str,
    metrics: list,
) -> dict:
    """Run all metrics on a single chapter. Returns dict with all scores + reasons."""
    test_case = LLMTestCase(
        input=original_text,
        actual_output=distilled_text,
    )

    results = {}
    for metric in metrics:
        name = getattr(metric, "name", None) or metric.__class__.__name__
        # SummarizationMetric doesn't have a custom name; use "Completeness"
        if isinstance(metric, SummarizationMetric):
            name = "Completeness"

        try:
            metric.measure(test_case)
            results[name] = {
                "score": metric.score,
                "reason": metric.reason,
            }
        except Exception:
            log.exception("Metric '%s' failed", name)
            results[name] = {
                "score": None,
                "reason": "metric evaluation failed",
            }

    return results
