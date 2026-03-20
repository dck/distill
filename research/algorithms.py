"""Distillation algorithms for the research pipeline."""

import time
from dataclasses import dataclass, field
from typing import Callable

from prompts import (
    get_distill_user_message,
    get_extract_prompt,
    get_refinement_prompt,
    get_rewrite_prompt,
    get_summary_prompt,
    get_system_prompt,
)


class SkipExperiment(Exception):
    """Raised when model context window is too small for an algorithm."""
    pass


@dataclass
class AlgorithmResult:
    chapters: dict[str, str]      # chapter_name -> distilled text
    metadata: dict = field(default_factory=dict)


# Signature: (model_id, system_prompt, user_message, temperature) -> (response_text, usage_dict)
LLMCaller = Callable[[str, str, str, float], tuple[str, dict]]


def _estimate_tokens(text: str) -> int:
    return len(text) // 4


def _check_context(model_config: dict, input_tokens: int) -> None:
    output_buffer = max(int(input_tokens * 0.5), 4000)
    if model_config["context_window"] < input_tokens + output_buffer:
        raise SkipExperiment(
            f"Context window {model_config['context_window']} too small for "
            f"{input_tokens} input + {output_buffer} buffer"
        )


def _accumulate_usage(metadata: dict, usage: dict) -> None:
    metadata["total_input_tokens"] = metadata.get("total_input_tokens", 0) + usage.get("input_tokens", 0)
    metadata["total_output_tokens"] = metadata.get("total_output_tokens", 0) + usage.get("output_tokens", 0)


def _join_distilled_chapters(chapters: dict[str, str]) -> str:
    """Build a cumulative distilled context from all completed chapter outputs."""
    return "\n\n".join(f"# {name}\n\n{text}" for name, text in chapters.items())


# ---------------------------------------------------------------------------
# 1. whole_book
# ---------------------------------------------------------------------------

def _whole_book(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    full_text = "\n\n".join(
        f"# {name}\n\n{text}" for name, text in chapters.items()
    )
    input_tokens = _estimate_tokens(full_text)
    _check_context(model_config, input_tokens)

    system = get_system_prompt(model_config["tier"])
    user_msg = get_distill_user_message(full_text)

    t0 = time.time()
    response, usage = call_llm(model_config["id"], system, user_msg, temperature)
    elapsed = time.time() - t0

    metadata = {"elapsed_seconds": elapsed}
    _accumulate_usage(metadata, usage)

    return AlgorithmResult(
        chapters={"__whole_book__.txt": response},
        metadata=metadata,
    )


# ---------------------------------------------------------------------------
# 2. independent
# ---------------------------------------------------------------------------

def _independent(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    system = get_system_prompt(model_config["tier"])
    results = {}
    metadata: dict = {}
    t0 = time.time()

    for name, text in chapters.items():
        input_tokens = _estimate_tokens(text)
        _check_context(model_config, input_tokens)

        user_msg = get_distill_user_message(text)
        response, usage = call_llm(model_config["id"], system, user_msg, temperature)
        results[name] = response
        _accumulate_usage(metadata, usage)

    metadata["elapsed_seconds"] = time.time() - t0
    return AlgorithmResult(chapters=results, metadata=metadata)


# ---------------------------------------------------------------------------
# 3 & 4. overlap_10 / overlap_20
# ---------------------------------------------------------------------------

def _overlap(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
    pct: float,
) -> AlgorithmResult:
    system = get_system_prompt(model_config["tier"])
    results = {}
    metadata: dict = {}
    t0 = time.time()

    prev_raw: str | None = None
    for name, text in chapters.items():
        context = None
        if prev_raw is not None:
            overlap_len = int(len(prev_raw) * pct)
            context = f"CONTEXT (end of previous chapter):\n\n{prev_raw[-overlap_len:]}"

        combined = (context or "") + text
        input_tokens = _estimate_tokens(combined)
        _check_context(model_config, input_tokens)

        user_msg = get_distill_user_message(text, context=context)
        response, usage = call_llm(model_config["id"], system, user_msg, temperature)
        results[name] = response
        _accumulate_usage(metadata, usage)
        prev_raw = text

    metadata["elapsed_seconds"] = time.time() - t0
    return AlgorithmResult(chapters=results, metadata=metadata)


def _overlap_10(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    return _overlap(chapters, model_config, call_llm, temperature, pct=0.10)


def _overlap_20(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    return _overlap(chapters, model_config, call_llm, temperature, pct=0.20)


# ---------------------------------------------------------------------------
# 5. running_summary
# ---------------------------------------------------------------------------

def _running_summary(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    system = get_system_prompt(model_config["tier"])
    summary_system = get_summary_prompt()
    results = {}
    metadata: dict = {}
    t0 = time.time()

    cumulative_summaries: list[str] = []

    for name, text in chapters.items():
        context = None
        if cumulative_summaries:
            context = "SUMMARIES OF PREVIOUS CHAPTERS:\n\n" + "\n\n---\n\n".join(cumulative_summaries)

        combined = (context or "") + text
        input_tokens = _estimate_tokens(combined)
        _check_context(model_config, input_tokens)

        # Distill the chapter
        user_msg = get_distill_user_message(text, context=context)
        response, usage = call_llm(model_config["id"], system, user_msg, temperature)
        results[name] = response
        _accumulate_usage(metadata, usage)

        # Generate summary of the distilled chapter
        summary_response, summary_usage = call_llm(
            model_config["id"], summary_system, response, temperature
        )
        _accumulate_usage(metadata, summary_usage)
        cumulative_summaries.append(f"**{name}**: {summary_response}")

    metadata["elapsed_seconds"] = time.time() - t0
    return AlgorithmResult(chapters=results, metadata=metadata)


# ---------------------------------------------------------------------------
# 6. hierarchical
# ---------------------------------------------------------------------------

def _hierarchical(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    system = get_system_prompt(model_config["tier"])
    metadata: dict = {}
    t0 = time.time()

    # Pass 1: independent distillation
    pass1_results = {}
    for name, text in chapters.items():
        input_tokens = _estimate_tokens(text)
        _check_context(model_config, input_tokens)

        user_msg = get_distill_user_message(text)
        response, usage = call_llm(model_config["id"], system, user_msg, temperature)
        pass1_results[name] = response
        _accumulate_usage(metadata, usage)

    # Pass 2: coherence refinement
    combined_distilled = "\n\n".join(
        f"# {name}\n\n{text}" for name, text in pass1_results.items()
    )
    input_tokens = _estimate_tokens(combined_distilled)
    _check_context(model_config, input_tokens)

    refinement_system = get_refinement_prompt()
    response, usage = call_llm(model_config["id"], refinement_system, combined_distilled, temperature)
    _accumulate_usage(metadata, usage)

    metadata["elapsed_seconds"] = time.time() - t0
    return AlgorithmResult(
        chapters={"__hierarchical__.txt": response},
        metadata=metadata,
    )


# ---------------------------------------------------------------------------
# 7. incremental
# ---------------------------------------------------------------------------

def _incremental(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    system = get_system_prompt(model_config["tier"])
    metadata: dict = {}
    t0 = time.time()

    accumulated_distilled = ""
    results = {}

    for name, text in chapters.items():
        if accumulated_distilled:
            context = f"DISTILLED SO FAR:\n\n{accumulated_distilled}"
            combined = context + text
        else:
            context = None
            combined = text

        input_tokens = _estimate_tokens(combined)
        _check_context(model_config, input_tokens)

        user_msg = get_distill_user_message(text, context=context)
        response, usage = call_llm(model_config["id"], system, user_msg, temperature)
        results[name] = response
        accumulated_distilled = _join_distilled_chapters(results)
        _accumulate_usage(metadata, usage)

    metadata["elapsed_seconds"] = time.time() - t0
    return AlgorithmResult(chapters=results, metadata=metadata)


# ---------------------------------------------------------------------------
# 8. extract_compress
# ---------------------------------------------------------------------------

def _extract_compress(
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float,
) -> AlgorithmResult:
    extract_system = get_extract_prompt()
    rewrite_system = get_rewrite_prompt()
    results = {}
    metadata: dict = {}
    t0 = time.time()

    for name, text in chapters.items():
        input_tokens = _estimate_tokens(text)
        _check_context(model_config, input_tokens)

        # Phase 1: extract structured elements
        extracted, usage1 = call_llm(model_config["id"], extract_system, text, temperature)
        _accumulate_usage(metadata, usage1)

        # Phase 2: rewrite into flowing prose
        rewrite_tokens = _estimate_tokens(extracted)
        _check_context(model_config, rewrite_tokens)

        rewritten, usage2 = call_llm(model_config["id"], rewrite_system, extracted, temperature)
        _accumulate_usage(metadata, usage2)

        results[name] = rewritten

    metadata["elapsed_seconds"] = time.time() - t0
    return AlgorithmResult(chapters=results, metadata=metadata)


# ---------------------------------------------------------------------------
# Dispatcher
# ---------------------------------------------------------------------------

_ALGORITHMS = {
    "whole_book": _whole_book,
    "independent": _independent,
    "overlap_10": _overlap_10,
    "overlap_20": _overlap_20,
    "running_summary": _running_summary,
    "hierarchical": _hierarchical,
    "incremental": _incremental,
    "extract_compress": _extract_compress,
}


def run_algorithm(
    algo_name: str,
    chapters: dict[str, str],
    model_config: dict,
    call_llm: LLMCaller,
    temperature: float = 0.3,
) -> AlgorithmResult:
    """Dispatch to the correct algorithm."""
    if algo_name not in _ALGORITHMS:
        raise ValueError(f"Unknown algorithm: {algo_name!r}. Available: {list(_ALGORITHMS.keys())}")
    return _ALGORITHMS[algo_name](chapters, model_config, call_llm, temperature)
