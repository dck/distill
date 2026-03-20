"""Generate markdown report + chart PNGs from evaluation data."""

import json
import tomllib
from collections import defaultdict
from datetime import date
from pathlib import Path

from charts import (
    generate_algo_comparison_chart,
    generate_compression_scatter,
    generate_cost_quality_scatter,
    generate_leaderboard_chart,
    generate_model_comparison_chart,
)
from eval_metrics import get_metric_weights

CONFIG_PATH = Path(__file__).parent / "config.toml"

METRIC_KEYS = ["Completeness", "Structure Preservation", "Coherence", "Compression Quality"]


def _load_config() -> dict:
    with open(CONFIG_PATH, "rb") as f:
        return tomllib.load(f)


def _model_pricing(config: dict) -> dict[str, dict]:
    """Return {model_id: {"cost_input": ..., "cost_output": ..., "free": ...}}."""
    pricing = {}
    for m in config.get("models", []):
        pricing[m["id"]] = {
            "cost_input": m.get("cost_input", 0),
            "cost_output": m.get("cost_output", 0),
            "free": m.get("free", False),
            "name": m["name"],
        }
    return pricing


def _estimate_cost(
    exp: dict, pricing: dict[str, dict],
) -> float:
    """Estimate cost for an experiment based on chapter sizes and model pricing."""
    model_id = exp.get("model_id", "")
    info = pricing.get(model_id)
    if not info or info["free"]:
        return 0.0

    metadata = exp.get("metadata", {})
    input_tokens = metadata.get("total_input_tokens")
    output_tokens = metadata.get("total_output_tokens")
    if input_tokens is not None or output_tokens is not None:
        return (
            (float(input_tokens or 0) * info["cost_input"] / 1_000_000)
            + (float(output_tokens or 0) * info["cost_output"] / 1_000_000)
        )

    chapters = exp.get("chapters", {})
    if not chapters:
        return 0.0

    total = 0.0
    for ch_data in chapters.values():
        input_chars = ch_data.get("input_chars", 0)
        output_chars = ch_data.get("output_chars", 0)
        # Estimate tokens as chars / 4
        input_cost = (input_chars / 4) * info["cost_input"] / 1_000_000
        output_cost = (output_chars / 4) * info["cost_output"] / 1_000_000
        total += input_cost + output_cost

    return total


def _estimate_cost_per_chapter(
    exp: dict, pricing: dict[str, dict],
) -> float:
    """Estimate cost per chapter for an experiment."""
    chapters = exp.get("chapters", {})
    if not chapters:
        return 0.0
    total = _estimate_cost(exp, pricing)
    return total / len(chapters)


def _safe_avg(values: list[float | None]) -> float:
    """Average ignoring None values."""
    clean = [v for v in values if v is not None]
    return sum(clean) / len(clean) if clean else 0.0


def generate_report(eval_report_path: Path, reports_dir: Path) -> None:
    """Generate markdown report + chart PNGs from eval data."""
    config = _load_config()
    weights = get_metric_weights(config)
    pricing = _model_pricing(config)

    with open(eval_report_path) as f:
        data = json.load(f)

    experiments = data.get("experiments", {})
    if not experiments:
        print("No experiments found in eval report.")
        return

    # Collect unique models and algorithms
    models = set()
    algorithms = set()
    for exp in experiments.values():
        models.add(exp.get("model", "Unknown"))
        algorithms.add(exp.get("algorithm", "Unknown"))

    # --- Prepare leaderboard rows ---
    leaderboard = []
    for key, exp in experiments.items():
        row = {
            "key": key,
            "model": exp.get("model", "Unknown"),
            "model_id": exp.get("model_id", ""),
            "algorithm": exp.get("algorithm", "Unknown"),
            "composite_score": exp.get("composite_score", 0.0),
            "avg_compression_ratio": exp.get("avg_compression_ratio", 0.0),
        }
        avgs = exp.get("averages", {})
        for mk in METRIC_KEYS:
            row[mk] = avgs.get(mk, 0.0)
        row["cost"] = _estimate_cost(exp, pricing)
        row["cost_per_chapter"] = _estimate_cost_per_chapter(exp, pricing)
        leaderboard.append(row)

    leaderboard.sort(key=lambda r: r["composite_score"], reverse=True)

    # --- Prepare chart data ---
    charts_dir = reports_dir / "charts"
    charts_dir.mkdir(parents=True, exist_ok=True)

    # 1. Leaderboard chart data
    leaderboard_chart_data = [
        {
            "experiment": f"{r['model']} x {r['algorithm']}",
            "composite_score": round(r["composite_score"], 3),
        }
        for r in leaderboard
    ]

    # 2. Algorithm comparison: group by algorithm, average metrics across models
    algo_groups: dict[str, list[dict]] = defaultdict(list)
    for r in leaderboard:
        algo_groups[r["algorithm"]].append(r)

    algo_chart_data = []
    for algo, rows in sorted(algo_groups.items()):
        algo_chart_data.append({
            "algorithm": algo,
            "completeness": round(_safe_avg([r["Completeness"] for r in rows]), 3),
            "structure": round(_safe_avg([r["Structure Preservation"] for r in rows]), 3),
            "coherence": round(_safe_avg([r["Coherence"] for r in rows]), 3),
            "compression_quality": round(_safe_avg([r["Compression Quality"] for r in rows]), 3),
        })

    # 3. Model comparison: group by model, average metrics across algorithms
    model_groups: dict[str, list[dict]] = defaultdict(list)
    for r in leaderboard:
        model_groups[r["model"]].append(r)

    model_chart_data = []
    for model, rows in sorted(model_groups.items()):
        model_chart_data.append({
            "model": model,
            "completeness": round(_safe_avg([r["Completeness"] for r in rows]), 3),
            "structure": round(_safe_avg([r["Structure Preservation"] for r in rows]), 3),
            "coherence": round(_safe_avg([r["Coherence"] for r in rows]), 3),
            "compression_quality": round(_safe_avg([r["Compression Quality"] for r in rows]), 3),
        })

    # 4. Compression scatter
    compression_scatter_data = [
        {
            "compression_ratio": round(r["avg_compression_ratio"], 3),
            "completeness": round(r["Completeness"], 3),
            "label": f"{r['model']} x {r['algorithm']}",
            "algorithm": r["algorithm"],
        }
        for r in leaderboard
    ]

    # 5. Cost quality scatter
    cost_quality_data = [
        {
            "cost": round(r["cost"], 4),
            "composite_score": round(r["composite_score"], 3),
            "label": f"{r['model']} x {r['algorithm']}",
            "model": r["model"],
        }
        for r in leaderboard
    ]

    # --- Generate charts ---
    chart_specs = [
        ("Generating leaderboard chart...", generate_leaderboard_chart, leaderboard_chart_data, charts_dir / "leaderboard.png"),
        ("Generating algorithm comparison chart...", generate_algo_comparison_chart, algo_chart_data, charts_dir / "algo_comparison.png"),
        ("Generating model comparison chart...", generate_model_comparison_chart, model_chart_data, charts_dir / "model_comparison.png"),
        ("Generating compression scatter chart...", generate_compression_scatter, compression_scatter_data, charts_dir / "compression_scatter.png"),
        ("Generating cost vs quality chart...", generate_cost_quality_scatter, cost_quality_data, charts_dir / "cost_quality.png"),
    ]

    for msg, fn, chart_data, path in chart_specs:
        print(msg)
        fn(chart_data, path)

    # --- Build algorithm analysis table ---
    algo_analysis = []
    for algo, rows in sorted(algo_groups.items()):
        algo_analysis.append({
            "algorithm": algo,
            "Completeness": _safe_avg([r["Completeness"] for r in rows]),
            "Structure Preservation": _safe_avg([r["Structure Preservation"] for r in rows]),
            "Coherence": _safe_avg([r["Coherence"] for r in rows]),
            "Compression Quality": _safe_avg([r["Compression Quality"] for r in rows]),
            "composite": _safe_avg([r["composite_score"] for r in rows]),
        })
    algo_analysis.sort(key=lambda r: r["composite"], reverse=True)
    best_algo = algo_analysis[0] if algo_analysis else None

    # --- Build model analysis table ---
    model_analysis = []
    for model, rows in sorted(model_groups.items()):
        model_analysis.append({
            "model": model,
            "Completeness": _safe_avg([r["Completeness"] for r in rows]),
            "Structure Preservation": _safe_avg([r["Structure Preservation"] for r in rows]),
            "Coherence": _safe_avg([r["Coherence"] for r in rows]),
            "Compression Quality": _safe_avg([r["Compression Quality"] for r in rows]),
            "composite": _safe_avg([r["composite_score"] for r in rows]),
        })
    model_analysis.sort(key=lambda r: r["composite"], reverse=True)

    # --- Build cost table ---
    # Aggregate cost per model (average across algorithms)
    model_cost: dict[str, list[dict]] = defaultdict(list)
    for r in leaderboard:
        model_cost[r["model"]].append(r)

    cost_rows = []
    for model, rows in sorted(model_cost.items()):
        avg_cost_per_ch = _safe_avg([r["cost_per_chapter"] for r in rows])
        total_cost = _safe_avg([r["cost"] for r in rows])
        cost_rows.append({
            "model": model,
            "cost_per_chapter": avg_cost_per_ch,
            "total_cost": total_cost,
        })
    cost_rows.sort(key=lambda r: r["total_cost"])

    # --- Recommendations ---
    # Best algorithm
    best_algo_name = best_algo["algorithm"] if best_algo else "N/A"
    best_algo_score = best_algo["composite"] if best_algo else 0.0

    # Best models by category
    free_models = [r for r in leaderboard if pricing.get(r["model_id"], {}).get("free") is True]
    paid_models = [r for r in leaderboard if pricing.get(r["model_id"], {}).get("free") is False]

    best_free = free_models[0] if free_models else None
    best_quality = leaderboard[0] if leaderboard else None

    # Best budget = best paid model (cheapest with decent quality)
    # Sort paid by cost_per_chapter ascending, pick first with composite > median
    if paid_models:
        paid_by_cost = sorted(paid_models, key=lambda r: r["cost_per_chapter"])
        best_budget = paid_by_cost[0]
    else:
        best_budget = None

    # Key insights
    insights = []

    if best_algo and algo_analysis:
        worst_algo = algo_analysis[-1]
        if worst_algo["algorithm"] != best_algo_name:
            diff = best_algo["composite"] - worst_algo["composite"]
            insights.append(
                f"**{best_algo_name}** is the top algorithm with a composite score of "
                f"{best_algo_score:.2f}, outperforming **{worst_algo['algorithm']}** by "
                f"{diff:.2f} points."
            )

    if model_analysis and len(model_analysis) > 1:
        best_m = model_analysis[0]
        worst_m = model_analysis[-1]
        insights.append(
            f"**{best_m['model']}** leads model rankings ({best_m['composite']:.2f}), "
            f"while **{worst_m['model']}** trails ({worst_m['composite']:.2f})."
        )

    if leaderboard:
        avg_ratio = _safe_avg([r["avg_compression_ratio"] for r in leaderboard])
        insights.append(
            f"Average compression ratio across all experiments: {avg_ratio:.2f} "
            f"(retaining {avg_ratio * 100:.0f}% of original text)."
        )

    if free_models and paid_models:
        free_avg = _safe_avg([r["composite_score"] for r in free_models])
        paid_avg = _safe_avg([r["composite_score"] for r in paid_models])
        insights.append(
            f"Free models average {free_avg:.2f} composite vs paid models at {paid_avg:.2f} "
            f"-- {'paid models justify their cost' if paid_avg > free_avg + 0.03 else 'free models are competitive'}."
        )

    if len(insights) < 3 and leaderboard:
        top = leaderboard[0]
        insights.append(
            f"Top experiment: **{top['model']} x {top['algorithm']}** "
            f"(composite: {top['composite_score']:.2f})."
        )

    # --- Write markdown ---
    print("Writing report.md...")
    lines = []

    # Header
    lines.append("# Distillation Research Report")
    lines.append("")
    lines.append(f"Generated: {date.today().isoformat()}")
    lines.append("")
    lines.append("## Overview")
    lines.append("")
    lines.append(
        f"{len(experiments)} experiments evaluated across {len(models)} models "
        f"and {len(algorithms)} algorithms."
    )
    weight_desc = ", ".join(
        f"{k} ({v})" for k, v in weights.items()
    )
    lines.append(f"Metric weights: {weight_desc}.")
    lines.append("")

    # Leaderboard
    lines.append("## Leaderboard")
    lines.append("")
    lines.append(
        "| Rank | Model | Algorithm | Composite | Completeness | Structure | "
        "Coherence | Compression Quality | Avg Compression Ratio |"
    )
    lines.append(
        "|------|-------|-----------|-----------|--------------|-----------|"
        "-----------|---------------------|-----------------------|"
    )
    for i, r in enumerate(leaderboard, 1):
        lines.append(
            f"| {i} | {r['model']} | {r['algorithm']} | "
            f"{r['composite_score']:.2f} | {r['Completeness']:.2f} | "
            f"{r['Structure Preservation']:.2f} | {r['Coherence']:.2f} | "
            f"{r['Compression Quality']:.2f} | {r['avg_compression_ratio']:.2f} |"
        )
    lines.append("")

    # Algorithm Analysis
    lines.append("## Algorithm Analysis")
    lines.append("")
    lines.append(
        "| Algorithm | Completeness | Structure | Coherence | "
        "Compression Quality | Composite |"
    )
    lines.append(
        "|-----------|--------------|-----------|-----------|"
        "---------------------|-----------|"
    )
    for r in algo_analysis:
        lines.append(
            f"| {r['algorithm']} | {r['Completeness']:.2f} | "
            f"{r['Structure Preservation']:.2f} | {r['Coherence']:.2f} | "
            f"{r['Compression Quality']:.2f} | {r['composite']:.2f} |"
        )
    lines.append("")
    if best_algo:
        lines.append(
            f"**Best algorithm: {best_algo_name}** (composite: {best_algo_score:.2f})"
        )
    lines.append("")

    # Model Analysis
    lines.append("## Model Analysis")
    lines.append("")
    lines.append(
        "| Model | Completeness | Structure | Coherence | "
        "Compression Quality | Composite |"
    )
    lines.append(
        "|-------|--------------|-----------|-----------|"
        "---------------------|-----------|"
    )
    for r in model_analysis:
        lines.append(
            f"| {r['model']} | {r['Completeness']:.2f} | "
            f"{r['Structure Preservation']:.2f} | {r['Coherence']:.2f} | "
            f"{r['Compression Quality']:.2f} | {r['composite']:.2f} |"
        )
    lines.append("")
    if model_analysis:
        best_m = model_analysis[0]
        lines.append(
            f"**Best model: {best_m['model']}** (composite: {best_m['composite']:.2f})"
        )
    lines.append("")

    # Cost Comparison
    lines.append("## Cost Comparison")
    lines.append("")
    lines.append("| Model | Cost per Chapter | Total Estimated Cost |")
    lines.append("|-------|------------------|----------------------|")
    for r in cost_rows:
        lines.append(
            f"| {r['model']} | ${r['cost_per_chapter']:.4f} | "
            f"${r['total_cost']:.4f} |"
        )
    lines.append("")

    # Charts
    lines.append("## Charts")
    lines.append("")
    lines.append("### Leaderboard")
    lines.append("![Leaderboard](charts/leaderboard.png)")
    lines.append("")
    lines.append("### Algorithm Comparison")
    lines.append("![Algorithm Comparison](charts/algo_comparison.png)")
    lines.append("")
    lines.append("### Model Comparison")
    lines.append("![Model Comparison](charts/model_comparison.png)")
    lines.append("")
    lines.append("### Compression vs Completeness")
    lines.append("![Compression Scatter](charts/compression_scatter.png)")
    lines.append("")
    lines.append("### Cost vs Quality")
    lines.append("![Cost vs Quality](charts/cost_quality.png)")
    lines.append("")

    # Recommendations
    lines.append("## Recommendations")
    lines.append("")
    lines.append("### Recommended Algorithm")
    lines.append("")
    if best_algo:
        lines.append(
            f"**{best_algo_name}** -- highest average composite score "
            f"({best_algo_score:.2f}) across all models."
        )
    lines.append("")

    lines.append("### Recommended Models")
    lines.append("")
    if best_free:
        lines.append(
            f"- **Best Free**: {best_free['model']} "
            f"(composite: {best_free['composite_score']:.2f})"
        )
    if best_budget:
        lines.append(
            f"- **Best Budget**: {best_budget['model']} "
            f"(composite: {best_budget['composite_score']:.2f}, "
            f"cost: ${best_budget['cost_per_chapter']:.4f}/chapter)"
        )
    if best_quality:
        lines.append(
            f"- **Best Quality**: {best_quality['model']} "
            f"(composite: {best_quality['composite_score']:.2f})"
        )
    lines.append("")

    lines.append("### Key Insights")
    lines.append("")
    for insight in insights:
        lines.append(f"- {insight}")
    lines.append("")

    lines.append("### Limitations")
    lines.append("")
    lines.append("- Single book (Think and Grow Rich) -- results may not generalize")
    lines.append("- 6 chapters -- small sample size")
    lines.append("- English only")
    lines.append("- Token estimation is approximate (len/4)")
    lines.append("")

    report_path = reports_dir / "report.md"
    report_path.write_text("\n".join(lines))
    print(f"Report written to {report_path}")
