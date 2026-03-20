"""CLI entry point for the distillation research pipeline."""

from __future__ import annotations

import argparse
import json
import logging
import os
import sys
import time
from pathlib import Path

import tomli
from dotenv import load_dotenv

ROOT = Path(__file__).parent
load_dotenv(ROOT / ".env")

from algorithms import AlgorithmResult, SkipExperiment, run_algorithm
log = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def load_config() -> dict:
    config_path = ROOT / "config.toml"
    with open(config_path, "rb") as f:
        return tomli.load(f)


def model_slug(model_id: str) -> str:
    """Convert model ID to filesystem-safe slug."""
    return model_id.replace("/", "_").replace(":", "_")


def load_chapters() -> dict[str, str]:
    """Load original chapters from data/originals/, sorted by filename."""
    originals = ROOT / "data" / "originals"
    if not originals.exists() or not any(originals.iterdir()):
        print(
            "No chapter files found in data/originals/.\n"
            "Run 'uv run python fetch_book.py' first.",
            file=sys.stderr,
        )
        sys.exit(1)

    chapters = {}
    for path in sorted(originals.glob("*.txt")):
        chapters[path.name] = path.read_text(encoding="utf-8")
    return chapters


def experiment_complete(experiment_dir: Path, chapter_names: list[str]) -> bool:
    for name in chapter_names:
        path = experiment_dir / name
        if not path.exists() or path.stat().st_size == 0:
            return False
    return True


def save_results(experiment_dir: Path, result: AlgorithmResult) -> None:
    experiment_dir.mkdir(parents=True, exist_ok=True)
    for name, text in result.chapters.items():
        (experiment_dir / name).write_text(text, encoding="utf-8")
    if result.metadata:
        (experiment_dir / "metadata.json").write_text(
            json.dumps(result.metadata, indent=2),
            encoding="utf-8",
        )


def find_model_config(config: dict, model_id: str) -> dict | None:
    for m in config["models"]:
        if m["id"] == model_id:
            return m
    return None


# ---------------------------------------------------------------------------
# LLM caller
# ---------------------------------------------------------------------------


def make_llm_caller(client, config: dict):
    settings = config["settings"]
    retry_attempts = settings["retry_attempts"]
    backoff_base = settings["retry_backoff_base"]
    free_delay = settings["free_model_delay_seconds"]

    if retry_attempts < 1:
        raise ValueError("config.settings.retry_attempts must be at least 1")

    # Track which models are free for delay logic
    free_models = {m["id"] for m in config["models"] if m.get("free", False)}

    def call_llm(
        model_id: str, system: str, user_message: str, temperature: float
    ) -> tuple[str, dict]:
        # Free model rate-limit delay
        if model_id in free_models:
            time.sleep(free_delay)

        last_err = None
        for attempt in range(retry_attempts):
            try:
                response = client.chat.completions.create(
                    model=model_id,
                    temperature=temperature,
                    messages=[
                        {"role": "system", "content": system},
                        {"role": "user", "content": user_message},
                    ],
                )
                text = response.choices[0].message.content
                usage = {}
                if response.usage:
                    usage = {
                        "input_tokens": response.usage.prompt_tokens,
                        "output_tokens": response.usage.completion_tokens,
                    }
                return text, usage
            except Exception as e:
                last_err = e
                if attempt < retry_attempts - 1:
                    wait = backoff_base ** (2 * attempt + 1)
                    log.warning(
                        "LLM call failed (attempt %d/%d): %s — retrying in %.0fs",
                        attempt + 1,
                        retry_attempts,
                        e,
                        wait,
                    )
                    time.sleep(wait)

        raise last_err  # type: ignore[misc]

    return call_llm


# ---------------------------------------------------------------------------
# distill
# ---------------------------------------------------------------------------


def cmd_distill(args: argparse.Namespace, config: dict) -> None:
    from openai import OpenAI

    chapters = load_chapters()
    chapter_names = list(chapters.keys())

    client = OpenAI(
        base_url="https://openrouter.ai/api/v1",
        api_key=os.environ["OPENROUTER_API_KEY"],
    )
    call_llm = make_llm_caller(client, config)
    temperature = config["settings"]["temperature"]
    algos = config["algorithms"]["enabled"]

    # Filter by CLI flags
    models = config["models"]
    if args.model:
        models = [m for m in models if m["id"] == args.model]
        if not models:
            print(f"Model '{args.model}' not found in config.toml", file=sys.stderr)
            sys.exit(1)
    if args.algo:
        if args.algo not in algos:
            print(f"Algorithm '{args.algo}' not in enabled list", file=sys.stderr)
            sys.exit(1)
        algos = [args.algo]

    results_dir = ROOT / "data" / "results"
    total = len(models) * len(algos)
    completed = 0
    skipped = 0
    failed_models: set[str] = set()
    # Track consecutive failures per model
    consec_failures: dict[str, int] = {}

    for model_config in models:
        mid = model_config["id"]
        slug = model_slug(mid)
        short_name = model_config["name"]

        if mid in failed_models:
            continue

        for algo in algos:
            completed += 1
            experiment_dir = results_dir / f"{slug}__{algo}"

            # Determine expected output files
            if algo == "whole_book":
                expected = ["__whole_book__.txt"]
            elif algo == "hierarchical":
                expected = ["__hierarchical__.txt"]
            else:
                expected = chapter_names

            # Checkpoint: skip if complete
            if experiment_complete(experiment_dir, expected):
                print(
                    f"[{completed}/{total}] {short_name} x {algo} — already complete, skipping"
                )
                continue

            # Skip if model is marked down
            if mid in failed_models:
                print(
                    f"[{completed}/{total}] {short_name} x {algo} — model down, skipping"
                )
                skipped += 1
                continue

            print(f"[{completed}/{total}] {short_name} x {algo} — running...")

            try:
                t0 = time.time()
                result = run_algorithm(algo, chapters, model_config, call_llm, temperature)
                elapsed = time.time() - t0
                save_results(experiment_dir, result)
                consec_failures[mid] = 0

                n_outputs = len(result.chapters)
                print(
                    f"[{completed}/{total}] {short_name} x {algo} -> "
                    f"{n_outputs} outputs ({elapsed:.1f}s)"
                )

            except SkipExperiment as e:
                print(
                    f"[{completed}/{total}] {short_name} x {algo} — skipped: {e}"
                )
                skipped += 1

            except Exception as e:
                log.exception("Experiment failed: %s x %s", short_name, algo)
                print(
                    f"[{completed}/{total}] {short_name} x {algo} — FAILED: {e}"
                )
                consec_failures[mid] = consec_failures.get(mid, 0) + 1
                if consec_failures[mid] >= 3:
                    print(
                        f"  {short_name} has failed 3 consecutive times — marking as down"
                    )
                    failed_models.add(mid)

    print(f"\nDone. {completed} experiments attempted, {skipped} skipped.")
    if failed_models:
        print(f"Models marked down: {', '.join(failed_models)}")


# ---------------------------------------------------------------------------
# eval
# ---------------------------------------------------------------------------


def cmd_eval(args: argparse.Namespace, config: dict) -> None:
    from eval_metrics import (
        compute_composite_score,
        compute_compression_ratio,
        evaluate_chapter,
        create_metrics,
        get_metric_weights,
    )
    from judge import OpusJudge

    originals_dir = ROOT / "data" / "originals"
    results_dir = ROOT / "data" / "results"
    reports_dir = ROOT / "reports"
    reports_dir.mkdir(parents=True, exist_ok=True)

    if not results_dir.exists():
        print("No results directory found. Run 'distill' first.", file=sys.stderr)
        sys.exit(1)

    # Discover experiments
    experiments = sorted(
        d.name for d in results_dir.iterdir() if d.is_dir()
    )
    if args.experiment:
        if args.experiment not in experiments:
            print(
                f"Experiment '{args.experiment}' not found in data/results/",
                file=sys.stderr,
            )
            sys.exit(1)
        experiments = [args.experiment]

    if not experiments:
        print("No experiments found in data/results/", file=sys.stderr)
        sys.exit(1)

    # Create judge and metrics
    judge = OpusJudge()
    metrics = create_metrics(judge)
    weights = get_metric_weights(config)

    # Load originals
    originals = load_chapters()

    eval_report: dict = {"experiments": {}}

    for exp_name in experiments:
        exp_dir = results_dir / exp_name

        # Parse model and algo from experiment name
        # Format: {model_slug}__{algo}
        parts = exp_name.split("__", 1)
        if len(parts) != 2:
            log.warning("Skipping unrecognized experiment dir: %s", exp_name)
            continue
        slug, algo = parts

        # Find model config by slug
        model_config = None
        for m in config["models"]:
            if model_slug(m["id"]) == slug:
                model_config = m
                break

        # Discover chapter files in experiment (exclude eval_ files)
        chapter_files = sorted(
            f.name
            for f in exp_dir.iterdir()
            if f.is_file() and f.suffix == ".txt" and not f.name.startswith("eval_")
        )

        if not chapter_files:
            continue

        chapter_scores: dict = {}

        for ch_file in chapter_files:
            ch_name = ch_file.replace(".txt", "")
            eval_path = exp_dir / f"eval_{ch_file}".replace(".txt", ".json")

            # Checkpoint: skip if eval already exists
            if eval_path.exists():
                print(f"  {exp_name}/{ch_file} — eval exists, loading")
                with open(eval_path) as f:
                    result = json.load(f)
                chapter_scores[ch_name] = result
                continue

            # Special keys don't map to individual original chapters
            if ch_file.startswith("__"):
                # For whole_book / hierarchical, concat all originals
                original_text = "\n\n".join(originals.values())
            elif ch_file in originals:
                original_text = originals[ch_file]
            else:
                log.warning("No original found for %s, skipping", ch_file)
                continue

            distilled_text = (exp_dir / ch_file).read_text(encoding="utf-8")

            print(f"  Evaluating {exp_name}/{ch_file}...")
            result = evaluate_chapter(original_text, distilled_text, metrics)
            result["compression_ratio"] = compute_compression_ratio(
                original_text, distilled_text
            )

            # Save eval JSON immediately
            with open(eval_path, "w") as f:
                json.dump(result, f, indent=2)

            chapter_scores[ch_name] = result

        if not chapter_scores:
            continue

        metadata_path = exp_dir / "metadata.json"
        metadata = {}
        if metadata_path.exists():
            with open(metadata_path, encoding="utf-8") as f:
                metadata = json.load(f)

        # Aggregate scores for this experiment
        score_sums: dict[str, float] = {}
        score_counts: dict[str, int] = {}
        compression_ratios: list[float] = []

        for ch_name, ch_result in chapter_scores.items():
            cr = ch_result.get("compression_ratio")
            if cr is not None:
                compression_ratios.append(cr)
            for metric_name in weights:
                metric_data = ch_result.get(metric_name, {})
                score = metric_data.get("score") if isinstance(metric_data, dict) else None
                if score is not None:
                    score_sums[metric_name] = score_sums.get(metric_name, 0) + score
                    score_counts[metric_name] = score_counts.get(metric_name, 0) + 1

        averages = {
            k: score_sums[k] / score_counts[k]
            for k in weights
            if score_counts.get(k, 0) > 0
        }
        composite = compute_composite_score(averages, weights)
        avg_cr = sum(compression_ratios) / len(compression_ratios) if compression_ratios else 0.0

        # Build chapter data for report (scores only, not reasons)
        chapters_report = {}
        for ch_name, ch_result in chapter_scores.items():
            scores = {}
            for metric_name in weights:
                metric_data = ch_result.get(metric_name, {})
                if isinstance(metric_data, dict):
                    scores[metric_name] = metric_data.get("score")
            if ch_name.startswith("__"):
                original_for_sizes = "\n\n".join(originals.values())
            else:
                original_for_sizes = originals.get(f"{ch_name}.txt", "")
            distilled_name = f"{ch_name}.txt"
            distilled_path = exp_dir / distilled_name
            distilled_text = distilled_path.read_text(encoding="utf-8") if distilled_path.exists() else ""
            chapters_report[ch_name] = {
                "scores": scores,
                "compression_ratio": ch_result.get("compression_ratio"),
                "input_chars": len(original_for_sizes),
                "output_chars": len(distilled_text),
            }

        exp_entry = {
            "model": model_config["name"] if model_config else slug,
            "model_id": model_config["id"] if model_config else "",
            "model_slug": slug,
            "algorithm": algo,
            "chapters": chapters_report,
            "averages": averages,
            "composite_score": composite,
            "avg_compression_ratio": avg_cr,
            "metadata": metadata,
        }
        eval_report["experiments"][exp_name] = exp_entry

        print(
            f"  {exp_name}: composite={composite:.3f}, "
            f"compression={avg_cr:.3f}"
        )

    # Save aggregate report
    eval_report_path = reports_dir / "eval_report.json"
    with open(eval_report_path, "w") as f:
        json.dump(eval_report, f, indent=2)

    print(f"\nEval report saved to {eval_report_path}")


# ---------------------------------------------------------------------------
# report
# ---------------------------------------------------------------------------


def cmd_report(args: argparse.Namespace, config: dict) -> None:
    from report import generate_report

    reports_dir = ROOT / "reports"
    eval_report_path = reports_dir / "eval_report.json"
    if not eval_report_path.exists():
        print("No eval_report.json found. Run 'eval' first.")
        return
    generate_report(eval_report_path, reports_dir)


# ---------------------------------------------------------------------------
# status
# ---------------------------------------------------------------------------


def cmd_status(args: argparse.Namespace, config: dict) -> None:
    results_dir = ROOT / "data" / "results"
    originals = ROOT / "data" / "originals"

    # Get chapter names
    if originals.exists():
        chapter_names = sorted(f.name for f in originals.glob("*.txt"))
    else:
        chapter_names = []

    n_chapters = len(chapter_names) if chapter_names else "?"

    models = config["models"]
    algos = config["algorithms"]["enabled"]

    # Header
    print(
        f"{'Model':<20s} {'Algorithm':<20s} {'Chapters':>10s} "
        f"{'Evald':>8s} {'Status'}"
    )
    print("-" * 75)

    for model_config in models:
        mid = model_config["id"]
        slug = model_slug(mid)
        short_name = model_config["name"]

        for algo in algos:
            exp_dir = results_dir / f"{slug}__{algo}"

            if not exp_dir.exists():
                print(
                    f"{short_name:<20s} {algo:<20s} {'0/' + str(n_chapters):>10s} "
                    f"{'—':>8s} {'Not started'}"
                )
                continue

            # Determine expected files
            if algo == "whole_book":
                expected = ["__whole_book__.txt"]
            elif algo == "hierarchical":
                expected = ["__hierarchical__.txt"]
            else:
                expected = chapter_names

            # Count distilled chapter files
            distilled = [
                name for name in expected
                if (exp_dir / name).exists() and (exp_dir / name).stat().st_size > 0
            ]
            n_distilled = len(distilled)
            n_expected = len(expected)

            # Count eval files
            n_evald = 0
            for name in distilled:
                eval_name = f"eval_{name}".replace(".txt", ".json")
                if (exp_dir / eval_name).exists():
                    n_evald += 1

            # Determine status
            if n_distilled == 0:
                status = "Not started"
            elif n_distilled < n_expected:
                status = "Partial"
            elif n_evald == n_distilled:
                status = "Complete"
            else:
                status = "Needs eval"

            ch_str = f"{n_distilled}/{n_expected}"
            ev_str = f"{n_evald}/{n_distilled}" if n_distilled > 0 else "—"

            print(
                f"{short_name:<20s} {algo:<20s} {ch_str:>10s} "
                f"{ev_str:>8s} {status}"
            )


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Distillation research pipeline"
    )
    parser.add_argument(
        "-v", "--verbose", action="store_true", help="Enable debug logging"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # distill
    p_distill = subparsers.add_parser("distill", help="Run distillation")
    p_distill.add_argument("--model", type=str, help="Run only this model ID")
    p_distill.add_argument("--algo", type=str, help="Run only this algorithm")

    # eval
    p_eval = subparsers.add_parser("eval", help="Run evaluation")
    p_eval.add_argument(
        "--experiment", type=str, help="Evaluate only this experiment"
    )

    # report
    subparsers.add_parser("report", help="Generate report")

    # status
    subparsers.add_parser("status", help="Show experiment status")

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.WARNING,
        format="%(levelname)s: %(message)s",
    )

    config = load_config()

    commands = {
        "distill": cmd_distill,
        "eval": cmd_eval,
        "report": cmd_report,
        "status": cmd_status,
    }
    commands[args.command](args, config)


if __name__ == "__main__":
    main()
