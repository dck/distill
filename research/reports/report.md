# Distillation Research Report

Generated: 2026-03-22

## Overview

64 experiments evaluated across 8 models and 8 algorithms.
Metric weights: Completeness (0.35), Structure Preservation (0.25), Coherence (0.25), Compression Quality (0.15).

## Leaderboard

| Rank | Model | Algorithm | Composite | Completeness | Structure | Coherence | Compression Quality | Compression Ratio | tok/s |
|------|-------|-----------|-----------|--------------|-----------|-----------|---------------------|-------------------|-------|
| 1 | Gemini 2.5 Pro | whole_book | 0.94 | 0.99 | 1.00 | 0.90 | 0.80 | 0.66 | 146.0 |
| 2 | Trinity Large | hierarchical | 0.92 | 0.96 | 1.00 | 0.80 | 0.90 | 0.78 | 28.7 |
| 3 | DeepSeek V3.2 | whole_book | 0.92 | 0.95 | 0.90 | 0.90 | 0.90 | 0.74 | 48.2 |
| 4 | DeepSeek V3.2 | hierarchical | 0.92 | 0.87 | 1.00 | 0.90 | 0.90 | 0.78 | 30.8 |
| 5 | DeepSeek R1 | hierarchical | 0.91 | 0.99 | 1.00 | 0.70 | 0.90 | 0.27 | 55.0 |
| 6 | Grok 4.1 Fast | hierarchical | 0.90 | 0.94 | 1.00 | 0.70 | 1.00 | 0.30 | 146.0 |
| 7 | StepFun: Step 3.5 Flash | independent | 0.90 | 0.88 | 0.97 | 0.90 | 0.87 | 0.50 | 54.6 |
| 8 | MiniMax M2.5 | whole_book | 0.90 | 0.95 | 0.90 | 0.90 | 0.80 | 0.23 | 62.8 |
| 9 | Gemini 2.5 Pro | hierarchical | 0.90 | 0.98 | 0.90 | 0.80 | 0.90 | 0.70 | 121.7 |
| 10 | StepFun: Step 3.5 Flash | hierarchical | 0.90 | 0.94 | 0.90 | 0.90 | 0.80 | 0.46 | 67.4 |
| 11 | Nemotron 120B | independent | 0.89 | 0.91 | 0.95 | 0.85 | 0.82 | 0.36 | 24.1 |
| 12 | StepFun: Step 3.5 Flash | whole_book | 0.89 | 0.82 | 1.00 | 0.80 | 1.00 | 0.26 | 68.3 |
| 13 | StepFun: Step 3.5 Flash | overlap_20 | 0.89 | 0.86 | 0.92 | 0.93 | 0.82 | 0.53 | 59.0 |
| 14 | Nemotron 120B | running_summary | 0.88 | 0.87 | 0.98 | 0.82 | 0.87 | 0.35 | 22.7 |
| 15 | StepFun: Step 3.5 Flash | running_summary | 0.88 | 0.87 | 0.90 | 0.88 | 0.88 | 0.54 | 46.9 |
| 16 | MiniMax M2.5 | running_summary | 0.88 | 0.94 | 0.85 | 0.90 | 0.77 | 0.70 | 34.4 |
| 17 | StepFun: Step 3.5 Flash | overlap_10 | 0.87 | 0.83 | 0.93 | 0.87 | 0.88 | 0.43 | 63.6 |
| 18 | Grok 4.1 Fast | overlap_20 | 0.87 | 0.83 | 0.95 | 0.80 | 0.95 | 0.30 | 124.7 |
| 19 | Nemotron 120B | incremental | 0.87 | 0.82 | 0.95 | 0.87 | 0.83 | 0.33 | 19.0 |
| 20 | StepFun: Step 3.5 Flash | incremental | 0.86 | 0.83 | 0.92 | 0.87 | 0.85 | 0.45 | 51.3 |
| 21 | Grok 4.1 Fast | independent | 0.86 | 0.82 | 0.93 | 0.83 | 0.88 | 0.29 | 123.2 |
| 22 | Grok 4.1 Fast | overlap_10 | 0.85 | 0.77 | 0.95 | 0.83 | 0.92 | 0.28 | 119.0 |
| 23 | Nemotron 120B | overlap_20 | 0.85 | 0.79 | 0.97 | 0.82 | 0.83 | 0.36 | 26.0 |
| 24 | MiniMax M2.5 | overlap_10 | 0.85 | 0.85 | 0.80 | 0.92 | 0.80 | 0.64 | 43.6 |
| 25 | StepFun: Step 3.5 Flash | extract_compress | 0.85 | 0.81 | 0.88 | 0.88 | 0.82 | 0.41 | 59.4 |
| 26 | Grok 4.1 Fast | running_summary | 0.85 | 0.77 | 0.93 | 0.83 | 0.90 | 0.27 | 114.3 |
| 27 | Grok 4.1 Fast | incremental | 0.84 | 0.85 | 0.88 | 0.77 | 0.85 | 0.24 | 116.7 |
| 28 | MiniMax M2.5 | overlap_20 | 0.83 | 0.79 | 0.82 | 0.90 | 0.82 | 0.61 | 23.7 |
| 29 | Nemotron 120B | extract_compress | 0.82 | 0.83 | 0.87 | 0.73 | 0.88 | 0.57 | 36.6 |
| 30 | Grok 4.1 Fast | whole_book | 0.81 | 0.80 | 0.90 | 0.70 | 0.90 | 0.11 | 106.7 |
| 31 | Nemotron 120B | overlap_10 | 0.81 | 0.73 | 0.85 | 0.88 | 0.83 | 0.35 | 28.7 |
| 32 | Nemotron 120B | hierarchical | 0.81 | 0.79 | 1.00 | 0.60 | 0.90 | 0.34 | 26.9 |
| 33 | MiniMax M2.5 | hierarchical | 0.81 | 0.98 | 0.60 | 0.90 | 0.60 | 0.55 | 64.2 |
| 34 | Grok 4.1 Fast | extract_compress | 0.80 | 0.77 | 0.87 | 0.70 | 0.90 | 0.31 | 127.4 |
| 35 | Nemotron 120B | whole_book | 0.80 | 0.60 | 0.90 | 0.90 | 0.90 | 0.10 | 63.8 |
| 36 | MiniMax M2.5 | independent | 0.74 | 0.59 | 0.85 | 0.90 | 0.67 | 0.67 | 55.1 |
| 37 | Gemini 2.5 Pro | independent | 0.73 | 0.41 | 0.95 | 0.90 | 0.85 | 0.74 | 121.8 |
| 38 | DeepSeek R1 | whole_book | 0.71 | 0.48 | 0.90 | 0.80 | 0.80 | 0.07 | 15.6 |
| 39 | Gemini 2.5 Pro | running_summary | 0.71 | 0.32 | 0.97 | 0.90 | 0.87 | 0.76 | 119.1 |
| 40 | DeepSeek V3.2 | running_summary | 0.69 | 0.32 | 0.92 | 0.90 | 0.80 | 0.70 | 14.6 |
| 41 | DeepSeek V3.2 | incremental | 0.68 | 0.32 | 0.92 | 0.90 | 0.78 | 0.86 | 40.4 |
| 42 | Trinity Large | incremental | 0.67 | 0.32 | 0.87 | 0.90 | 0.78 | 0.88 | 24.6 |
| 43 | MiniMax M2.5 | incremental | 0.67 | 0.31 | 0.90 | 0.90 | 0.75 | 0.60 | 49.5 |
| 44 | DeepSeek V3.2 | independent | 0.66 | 0.33 | 0.88 | 0.90 | 0.68 | 0.72 | 23.3 |
| 45 | DeepSeek V3.2 | overlap_10 | 0.66 | 0.32 | 0.87 | 0.88 | 0.75 | 0.73 | 17.8 |
| 46 | Trinity Large | overlap_20 | 0.66 | 0.31 | 0.88 | 0.88 | 0.72 | 0.82 | 27.1 |
| 47 | Gemini 2.5 Pro | incremental | 0.65 | 0.27 | 0.87 | 0.90 | 0.75 | 0.71 | 117.4 |
| 48 | Trinity Large | independent | 0.65 | 0.27 | 0.90 | 0.87 | 0.73 | 0.71 | 11.9 |
| 49 | Gemini 2.5 Pro | overlap_20 | 0.65 | 0.16 | 0.95 | 0.93 | 0.78 | 0.74 | 116.6 |
| 50 | Trinity Large | whole_book | 0.64 | 0.60 | 0.50 | 0.80 | 0.70 | 0.09 | 45.3 |
| 51 | DeepSeek V3.2 | overlap_20 | 0.64 | 0.25 | 0.85 | 0.92 | 0.73 | 0.74 | 15.9 |
| 52 | DeepSeek R1 | incremental | 0.64 | 0.25 | 0.85 | 0.90 | 0.75 | 0.23 | 56.7 |
| 53 | DeepSeek R1 | overlap_10 | 0.64 | 0.22 | 0.92 | 0.87 | 0.75 | 0.25 | 23.4 |
| 54 | MiniMax M2.5 | extract_compress | 0.63 | 0.25 | 0.87 | 0.87 | 0.75 | 0.55 | 33.4 |
| 55 | DeepSeek R1 | running_summary | 0.63 | 0.21 | 0.95 | 0.83 | 0.75 | 0.22 | 41.3 |
| 56 | DeepSeek V3.2 | extract_compress | 0.63 | 0.16 | 0.92 | 0.88 | 0.82 | 0.45 | 13.4 |
| 57 | Trinity Large | running_summary | 0.61 | 0.24 | 0.80 | 0.87 | 0.75 | 0.74 | 24.5 |
| 58 | Gemini 2.5 Pro | overlap_10 | 0.61 | 0.16 | 0.85 | 0.90 | 0.80 | 0.75 | 121.5 |
| 59 | DeepSeek R1 | independent | 0.60 | 0.11 | 0.92 | 0.87 | 0.80 | 0.28 | 25.2 |
| 60 | DeepSeek R1 | overlap_20 | 0.60 | 0.11 | 0.90 | 0.90 | 0.77 | 0.26 | 47.1 |
| 61 | Gemini 2.5 Pro | extract_compress | 0.59 | 0.16 | 0.87 | 0.88 | 0.67 | 0.49 | 107.5 |
| 62 | Trinity Large | overlap_10 | 0.59 | 0.24 | 0.77 | 0.87 | 0.65 | 0.73 | 12.0 |
| 63 | DeepSeek R1 | extract_compress | 0.58 | 0.11 | 0.87 | 0.85 | 0.78 | 0.26 | 26.0 |
| 64 | Trinity Large | extract_compress | 0.53 | 0.40 | 0.53 | 0.75 | 0.43 | 0.65 | 20.2 |

## Algorithm Analysis

| Algorithm | Completeness | Structure | Coherence | Compression Quality | Composite |
|-----------|--------------|-----------|-----------|---------------------|-----------|
| hierarchical | 0.93 | 0.93 | 0.79 | 0.86 | 0.88 |
| whole_book | 0.77 | 0.88 | 0.84 | 0.85 | 0.83 |
| running_summary | 0.57 | 0.91 | 0.87 | 0.82 | 0.77 |
| independent | 0.54 | 0.92 | 0.88 | 0.79 | 0.76 |
| overlap_20 | 0.51 | 0.90 | 0.89 | 0.80 | 0.75 |
| overlap_10 | 0.51 | 0.87 | 0.88 | 0.80 | 0.74 |
| incremental | 0.50 | 0.89 | 0.88 | 0.79 | 0.74 |
| extract_compress | 0.44 | 0.83 | 0.82 | 0.76 | 0.68 |

**Best algorithm: hierarchical** (composite: 0.88)

## Model Analysis

| Model | Completeness | Structure | Coherence | Compression Quality | Composite |
|-------|--------------|-----------|-----------|---------------------|-----------|
| StepFun: Step 3.5 Flash | 0.85 | 0.93 | 0.88 | 0.86 | 0.88 |
| Grok 4.1 Fast | 0.82 | 0.93 | 0.77 | 0.91 | 0.85 |
| Nemotron 120B | 0.79 | 0.93 | 0.81 | 0.86 | 0.84 |
| MiniMax M2.5 | 0.71 | 0.82 | 0.90 | 0.74 | 0.79 |
| DeepSeek V3.2 | 0.44 | 0.91 | 0.90 | 0.80 | 0.72 |
| Gemini 2.5 Pro | 0.43 | 0.92 | 0.89 | 0.80 | 0.72 |
| DeepSeek R1 | 0.31 | 0.91 | 0.84 | 0.79 | 0.66 |
| Trinity Large | 0.42 | 0.78 | 0.84 | 0.71 | 0.66 |

**Best model: StepFun: Step 3.5 Flash** (composite: 0.88)

## Throughput (output tok/s)

| Model | Avg tok/s | Min | Max |
|-------|-----------|-----|-----|
| DeepSeek R1 | 36.3 | 15.6 | 56.7 |
| DeepSeek V3.2 | 25.5 | 13.4 | 48.2 |
| Gemini 2.5 Pro | 121.4 | 107.5 | 146.0 |
| Grok 4.1 Fast | 122.2 | 106.7 | 146.0 |
| MiniMax M2.5 | 45.9 | 23.7 | 64.2 |
| Nemotron 120B | 31.0 | 19.0 | 63.8 |
| StepFun: Step 3.5 Flash | 58.8 | 46.9 | 68.3 |
| Trinity Large | 24.3 | 11.9 | 45.3 |

## Cost Comparison

| Model | Cost per Chapter | Total Estimated Cost |
|-------|------------------|----------------------|
| Nemotron 120B | $0.0000 | $0.0000 |
| StepFun: Step 3.5 Flash | $0.0000 | $0.0000 |
| Trinity Large | $0.0000 | $0.0000 |
| Grok 4.1 Fast | $0.0036 | $0.0804 |
| DeepSeek V3.2 | $0.0055 | $0.1112 |
| MiniMax M2.5 | $0.0086 | $0.1886 |
| DeepSeek R1 | $0.0105 | $0.2352 |
| Gemini 2.5 Pro | $0.1283 | $2.7139 |

## Charts

### Leaderboard
![Leaderboard](charts/leaderboard.png)

### Algorithm Comparison
![Algorithm Comparison](charts/algo_comparison.png)

### Model Comparison
![Model Comparison](charts/model_comparison.png)

### Compression vs Completeness
![Compression Scatter](charts/compression_scatter.png)

### Cost vs Quality
![Cost vs Quality](charts/cost_quality.png)

## Recommendations

### Recommended Algorithm

**hierarchical** -- highest average composite score (0.88) across all models.

### Recommended Models

- **Best Free**: Trinity Large (composite: 0.92)
- **Best Budget**: Grok 4.1 Fast (composite: 0.86, cost: $0.0013/chapter)
- **Best Quality**: Gemini 2.5 Pro (composite: 0.94)

### Key Insights

- **hierarchical** is the top algorithm with a composite score of 0.88, outperforming **extract_compress** by 0.20 points.
- **StepFun: Step 3.5 Flash** leads model rankings (0.88), while **Trinity Large** trails (0.66).
- Average compression ratio across all experiments: 0.49 (retaining 49% of original text).
- Free models average 0.79 composite vs paid models at 0.75 -- free models are competitive.

### Limitations

- Single book (Think and Grow Rich) -- results may not generalize
- 6 chapters -- small sample size
- English only
- Token estimation is approximate (len/4)
