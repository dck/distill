"""QuickChart.io client — generates chart PNGs via HTTP POST."""

from pathlib import Path

import httpx

QUICKCHART_URL = "https://quickchart.io/chart"

COLORS = [
    "#4E79A7",
    "#F28E2B",
    "#E15759",
    "#76B7B2",
    "#59A14F",
    "#EDC948",
    "#B07AA1",
    "#FF9DA7",
    "#9C755F",
    "#BAB0AC",
]

METRIC_COLORS = {
    "completeness": "#4E79A7",
    "structure": "#F28E2B",
    "coherence": "#E15759",
    "compression_quality": "#76B7B2",
}

METRIC_LABELS = {
    "completeness": "Completeness",
    "structure": "Structure",
    "coherence": "Coherence",
    "compression_quality": "Compression Quality",
}


def _post_chart(chart_config: dict, output_path: Path) -> None:
    """POST chart config to QuickChart.io, save PNG."""
    payload = {
        "width": 1000,
        "height": 600,
        "devicePixelRatio": 2,
        "format": "png",
        "backgroundColor": "white",
        "chart": chart_config,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    resp = httpx.post(QUICKCHART_URL, json=payload, timeout=30)
    if resp.status_code != 200:
        raise RuntimeError(
            f"QuickChart returned {resp.status_code}: {resp.text[:500]}"
        )
    output_path.write_bytes(resp.content)


def _gradient(base: str, n: int) -> list[str]:
    """Generate n colors from base with decreasing opacity via rgba."""
    # Parse hex to rgb
    r, g, b = int(base[1:3], 16), int(base[3:5], 16), int(base[5:7], 16)
    return [f"rgba({r}, {g}, {b}, {1.0 - 0.5 * i / max(n - 1, 1):.2f})" for i in range(n)]


def generate_leaderboard_chart(data: list[dict], output_path: Path) -> None:
    """Horizontal bar chart of top 15 experiments by composite score."""
    sorted_data = sorted(data, key=lambda d: d["composite_score"], reverse=True)[:15]
    labels = [d["experiment"] for d in sorted_data]
    scores = [d["composite_score"] for d in sorted_data]
    colors = _gradient("#4E79A7", len(scores))

    config = {
        "type": "horizontalBar",
        "data": {
            "labels": labels,
            "datasets": [
                {
                    "label": "Composite Score",
                    "data": scores,
                    "backgroundColor": colors,
                }
            ],
        },
        "options": {
            "title": {"display": True, "text": "Top 15 Experiments by Composite Score"},
            "scales": {
                "xAxes": [{"ticks": {"beginAtZero": True}, "scaleLabel": {"display": True, "labelString": "Composite Score"}}],
                "yAxes": [{"scaleLabel": {"display": True, "labelString": "Experiment"}}],
            },
            "legend": {"display": False},
            "plugins": {
                "datalabels": {
                    "anchor": "end",
                    "align": "end",
                    "formatter": "(val) => val.toFixed(2)",
                },
            },
        },
    }
    _post_chart(config, output_path)


def generate_algo_comparison_chart(data: list[dict], output_path: Path) -> None:
    """Grouped bar chart: algorithms on x-axis, 4 metrics as bar groups."""
    labels = [d["algorithm"] for d in data]
    datasets = [
        {
            "label": METRIC_LABELS[metric],
            "data": [d[metric] for d in data],
            "backgroundColor": color,
        }
        for metric, color in METRIC_COLORS.items()
    ]

    config = {
        "type": "bar",
        "data": {"labels": labels, "datasets": datasets},
        "options": {
            "title": {"display": True, "text": "Algorithm Comparison"},
            "scales": {
                "xAxes": [{"scaleLabel": {"display": True, "labelString": "Algorithm"}}],
                "yAxes": [{"ticks": {"beginAtZero": True}, "scaleLabel": {"display": True, "labelString": "Score"}}],
            },
            "plugins": {
                "datalabels": {"display": False},
            },
        },
    }
    _post_chart(config, output_path)


def generate_model_comparison_chart(data: list[dict], output_path: Path) -> None:
    """Grouped bar chart: models on x-axis, 4 metrics as bar groups."""
    labels = [d["model"] for d in data]
    datasets = [
        {
            "label": METRIC_LABELS[metric],
            "data": [d[metric] for d in data],
            "backgroundColor": color,
        }
        for metric, color in METRIC_COLORS.items()
    ]

    config = {
        "type": "bar",
        "data": {"labels": labels, "datasets": datasets},
        "options": {
            "title": {"display": True, "text": "Model Comparison"},
            "scales": {
                "xAxes": [{"scaleLabel": {"display": True, "labelString": "Model"}}],
                "yAxes": [{"ticks": {"beginAtZero": True}, "scaleLabel": {"display": True, "labelString": "Score"}}],
            },
            "plugins": {
                "datalabels": {"display": False},
            },
        },
    }
    _post_chart(config, output_path)


def generate_compression_scatter(data: list[dict], output_path: Path) -> None:
    """Scatter plot: compression ratio (x) vs completeness score (y), colored by algorithm."""
    algo_groups: dict[str, list[dict]] = {}
    for d in data:
        algo_groups.setdefault(d["algorithm"], []).append(d)

    datasets = []
    for i, (algo, points) in enumerate(sorted(algo_groups.items())):
        color = COLORS[i % len(COLORS)]
        datasets.append({
            "label": algo,
            "data": [{"x": p["compression_ratio"], "y": p["completeness"]} for p in points],
            "backgroundColor": color,
            "pointRadius": 6,
        })

    config = {
        "type": "scatter",
        "data": {"datasets": datasets},
        "options": {
            "title": {"display": True, "text": "Compression Ratio vs Completeness"},
            "scales": {
                "xAxes": [{"scaleLabel": {"display": True, "labelString": "Compression Ratio"}}],
                "yAxes": [{"scaleLabel": {"display": True, "labelString": "Completeness Score"}}],
            },
            "plugins": {
                "datalabels": {
                    "display": True,
                    "align": "top",
                    "formatter": "(val, ctx) => ctx.dataset.data[ctx.dataIndex].label || ''",
                },
            },
        },
    }

    # Inject labels into point data for datalabels plugin
    for i, (algo, points) in enumerate(sorted(algo_groups.items())):
        for j, p in enumerate(points):
            config["data"]["datasets"][i]["data"][j]["label"] = p.get("label", "")

    _post_chart(config, output_path)


def generate_cost_quality_scatter(data: list[dict], output_path: Path) -> None:
    """Scatter plot: estimated cost (x) vs composite score (y), colored by model."""
    model_groups: dict[str, list[dict]] = {}
    for d in data:
        model_groups.setdefault(d["model"], []).append(d)

    datasets = []
    for i, (model, points) in enumerate(sorted(model_groups.items())):
        color = COLORS[i % len(COLORS)]
        datasets.append({
            "label": model,
            "data": [{"x": p["cost"], "y": p["composite_score"]} for p in points],
            "backgroundColor": color,
            "pointRadius": 6,
        })

    config = {
        "type": "scatter",
        "data": {"datasets": datasets},
        "options": {
            "title": {"display": True, "text": "Cost vs Quality"},
            "scales": {
                "xAxes": [{"scaleLabel": {"display": True, "labelString": "Estimated Cost ($)"}}],
                "yAxes": [{"scaleLabel": {"display": True, "labelString": "Composite Score"}}],
            },
            "plugins": {
                "datalabels": {
                    "display": True,
                    "align": "top",
                    "formatter": "(val, ctx) => ctx.dataset.data[ctx.dataIndex].label || ''",
                },
            },
        },
    }

    # Inject labels into point data for datalabels plugin
    for i, (model, points) in enumerate(sorted(model_groups.items())):
        for j, p in enumerate(points):
            config["data"]["datasets"][i]["data"][j]["label"] = p.get("label", "")

    _post_chart(config, output_path)
