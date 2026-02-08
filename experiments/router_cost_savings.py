#!/usr/bin/env python3
"""
Experiment: Estimate cost savings from the intelligent model router.

Compares routing cost vs. always using the default (most expensive) model.
Uses a realistic prompt distribution across complexity tiers.

Usage:
    python experiments/router_cost_savings.py
"""

import json
import sys

try:
    import debot_rust
except ImportError:
    print("Error: debot_rust not available. Build with: pip install .")
    sys.exit(1)

# ---------------------------------------------------------------------------
# Pricing reference (USD per 1M output tokens)
# ---------------------------------------------------------------------------
PRICING = {
    "openai/gpt-3.5-turbo": 1.50,
    "openai/gpt-4o-mini": 0.60,
    "anthropic/claude-opus-4-5": 25.00,
    "openai/o3": 8.00,
}

# The default model users would use without the router
BASELINE_MODEL = "anthropic/claude-opus-4-5"
BASELINE_COST = PRICING[BASELINE_MODEL]

# ---------------------------------------------------------------------------
# Representative prompt dataset
# Each entry: (expected_tier, prompt, avg_output_tokens)
# avg_output_tokens estimates typical response length for this query type
# ---------------------------------------------------------------------------
PROMPTS = [
    # --- SIMPLE: greetings, factual Q&A, short answers ---
    ("SIMPLE", "hi", 50),
    ("SIMPLE", "what time is it?", 30),
    ("SIMPLE", "how are you?", 50),
    ("SIMPLE", "thanks!", 20),
    ("SIMPLE", "what's the weather like?", 80),
    ("SIMPLE", "who is the president of france?", 100),
    ("SIMPLE", "translate hello to spanish", 30),
    ("SIMPLE", "what does HTTP stand for?", 50),
    ("SIMPLE", "tell me a joke", 80),
    ("SIMPLE", "good morning", 30),
    ("SIMPLE", "what is python?", 120),
    ("SIMPLE", "define machine learning", 100),
    ("SIMPLE", "what's 15% of 200?", 30),
    ("SIMPLE", "when was linux created?", 80),
    ("SIMPLE", "summarize this in one sentence", 50),

    # --- MEDIUM: code snippets, multi-step tasks, structured output ---
    ("MEDIUM", "write a function to sort a list in python", 200),
    ("MEDIUM", "create a bash script to backup my home directory", 300),
    ("MEDIUM", "implement a binary search in javascript", 250),
    ("MEDIUM", "build a REST API endpoint with express.js, first set up the router then add validation", 400),
    ("MEDIUM", "create a dockerfile for a python flask app", 250),
    ("MEDIUM", "write an async function to fetch data from an API and parse the json response", 300),
    ("MEDIUM", "implement a class for a linked list with insert and delete methods", 400),
    ("MEDIUM", "create a yaml config file for a kubernetes deployment", 200),
    ("MEDIUM", "build a simple CLI tool with argparse in python", 350),
    ("MEDIUM", "write a function to validate email addresses using regex", 200),

    # --- COMPLEX: architecture, debugging, technical depth ---
    ("COMPLEX", "implement a distributed cache system with consistent hashing algorithm, first design the hash ring then build the node management", 800),
    ("COMPLEX", "build a kubernetes operator that manages custom resources, create the controller with async reconciliation and implement proper error handling", 1000),
    ("COMPLEX", "design and implement a real-time event processing pipeline, first create the message queue abstraction then implement the consumer with backpressure", 900),
    ("COMPLEX", "implement a B-tree data structure with concurrent access support, build the node splitting algorithm and create thread-safe insert operations", 800),
    ("COMPLEX", "create a distributed task scheduler with fault tolerance, implement the leader election algorithm and build the task assignment system", 900),

    # --- REASONING: proofs, formal logic, deep analysis ---
    ("REASONING", "prove that the halting problem is undecidable using formal diagonalization, step by step with a theorem statement", 1200),
    ("REASONING", "prove by induction that the sum of first n squares equals n(n+1)(2n+1)/6, step by step with formal verification", 800),
    ("REASONING", "prove that every continuous function on a closed interval is uniformly continuous, step by step using the theorem of Heine-Cantor", 1000),
]

# ---------------------------------------------------------------------------
# Simulated daily usage distribution
# Reflects a typical power user: many simple queries, fewer complex ones
# ---------------------------------------------------------------------------
DAILY_DISTRIBUTION = {
    "SIMPLE": 40,   # 40 simple queries/day
    "MEDIUM": 20,   # 20 medium queries/day
    "COMPLEX": 8,   # 8 complex queries/day
    "REASONING": 2,  # 2 reasoning queries/day
}

# ---------------------------------------------------------------------------
# Run experiment
# ---------------------------------------------------------------------------


def run_experiment():
    debot_rust.reset_router_metrics()

    print("=" * 70)
    print("  Router Cost Savings Experiment")
    print("=" * 70)

    # --- Phase 1: Route all prompts and collect decisions ---
    print("\n[Phase 1] Routing prompts through the auto router...\n")

    results = []
    for expected_tier, prompt, avg_tokens in PROMPTS:
        decision_json = debot_rust.route_text(prompt, 4096)
        dec = json.loads(decision_json)
        results.append({
            "prompt": prompt[:60] + ("..." if len(prompt) > 60 else ""),
            "expected_tier": expected_tier,
            "actual_tier": dec["tier"],
            "model": dec["model"],
            "confidence": dec["confidence"],
            "cost_per_m": dec["cost_estimate"],
            "avg_tokens": avg_tokens,
        })

    # --- Phase 2: Per-prompt results ---
    print(f"{'Prompt':<63} {'Expected':<11} {'Routed':<11} {'Model':<28} {'$/M':>6}")
    print("-" * 125)
    misrouted = 0
    for r in results:
        match = "  " if r["expected_tier"] == r["actual_tier"] else "!!"
        if r["expected_tier"] != r["actual_tier"]:
            misrouted += 1
        print(
            f"{match} {r['prompt']:<60} {r['expected_tier']:<11} {r['actual_tier']:<11} "
            f"{r['model']:<28} ${r['cost_per_m']:>5.2f}"
        )

    accuracy = (len(results) - misrouted) / len(results) * 100
    print(f"\nRouting accuracy: {len(results) - misrouted}/{len(results)} ({accuracy:.0f}%)")

    # --- Phase 3: Cost simulation ---
    print("\n" + "=" * 70)
    print("  Cost Simulation (daily usage)")
    print("=" * 70)

    # Group prompts by expected tier and compute average cost per tier
    tier_avg_cost = {}
    tier_avg_tokens = {}
    for tier in DAILY_DISTRIBUTION:
        tier_results = [r for r in results if r["expected_tier"] == tier]
        if tier_results:
            tier_avg_cost[tier] = sum(r["cost_per_m"] for r in tier_results) / len(tier_results)
            tier_avg_tokens[tier] = sum(r["avg_tokens"] for r in tier_results) / len(tier_results)

    print(f"\n{'Tier':<12} {'Queries/day':>12} {'Avg tokens':>12} {'Router $/M':>12} {'Baseline $/M':>14}")
    print("-" * 65)

    total_router_cost = 0.0
    total_baseline_cost = 0.0
    total_tokens = 0

    for tier, daily_count in DAILY_DISTRIBUTION.items():
        avg_tokens = tier_avg_tokens.get(tier, 200)
        router_cost_per_m = tier_avg_cost.get(tier, BASELINE_COST)
        tokens_per_day = daily_count * avg_tokens
        total_tokens += tokens_per_day

        daily_router = tokens_per_day / 1_000_000 * router_cost_per_m
        daily_baseline = tokens_per_day / 1_000_000 * BASELINE_COST

        total_router_cost += daily_router
        total_baseline_cost += daily_baseline

        print(
            f"{tier:<12} {daily_count:>12} {avg_tokens:>12.0f} "
            f"${router_cost_per_m:>11.2f} ${BASELINE_COST:>13.2f}"
        )

    print("-" * 65)
    print(f"{'TOTAL':<12} {sum(DAILY_DISTRIBUTION.values()):>12} {total_tokens:>12}")

    # --- Phase 4: Summary ---
    print("\n" + "=" * 70)
    print("  Results")
    print("=" * 70)

    daily_savings = total_baseline_cost - total_router_cost
    monthly_savings = daily_savings * 30
    pct_savings = (daily_savings / total_baseline_cost * 100) if total_baseline_cost > 0 else 0

    print(f"\n  Baseline model:     {BASELINE_MODEL}")
    print(f"  Baseline cost/M:    ${BASELINE_COST:.2f}")
    print(f"  Total tokens/day:   {total_tokens:,}")
    print(f"\n  Daily cost (baseline):   ${total_baseline_cost:.4f}")
    print(f"  Daily cost (router):     ${total_router_cost:.4f}")
    print(f"  Daily savings:           ${daily_savings:.4f} ({pct_savings:.1f}%)")
    print(f"\n  Monthly savings (30d):   ${monthly_savings:.4f} ({pct_savings:.1f}%)")
    print(f"  Annual savings (365d):   ${daily_savings * 365:.4f} ({pct_savings:.1f}%)")

    # Tier distribution with router
    print("\n  Router tier distribution:")
    metrics = json.loads(debot_rust.get_router_metrics())
    total_calls = metrics["total_calls"]
    for tier, count in sorted(metrics["tier_counts"].items()):
        pct = count / total_calls * 100
        bar = "\u2588" * int(pct / 2)
        print(f"    {tier:<12} {count:>3} ({pct:>5.1f}%) {bar}")

    # --- Phase 5: Ideal router comparison ---
    # Simulates what savings would look like if routing accuracy were 100%
    print("\n" + "=" * 70)
    print("  Ideal Router (100% accuracy) vs. Current Router")
    print("=" * 70)

    # Ideal: each tier uses its designated model at the correct price
    IDEAL_TIER_COST = {
        "SIMPLE": PRICING["openai/gpt-3.5-turbo"],
        "MEDIUM": PRICING["openai/gpt-4o-mini"],
        "COMPLEX": PRICING["anthropic/claude-opus-4-5"],
        "REASONING": PRICING["openai/o3"],
    }

    print(f"\n  {'Tier':<12} {'Queries':>8} {'Avg tok':>8} {'Baseline':>12} {'Current':>12} {'Ideal':>12}")
    print("  " + "-" * 68)

    total_ideal = 0.0
    total_current_detail = 0.0
    total_baseline_detail = 0.0

    for tier, daily_count in DAILY_DISTRIBUTION.items():
        avg_tok = tier_avg_tokens.get(tier, 200)
        tok_day = daily_count * avg_tok

        cost_baseline = tok_day / 1_000_000 * BASELINE_COST
        cost_current = tok_day / 1_000_000 * tier_avg_cost.get(tier, BASELINE_COST)
        cost_ideal = tok_day / 1_000_000 * IDEAL_TIER_COST[tier]

        total_baseline_detail += cost_baseline
        total_current_detail += cost_current
        total_ideal += cost_ideal

        print(
            f"  {tier:<12} {daily_count:>8} {avg_tok:>8.0f} "
            f"${cost_baseline:>11.6f} ${cost_current:>11.6f} ${cost_ideal:>11.6f}"
        )

    print("  " + "-" * 68)

    cur_sav = (1 - total_current_detail / total_baseline_detail) * 100 if total_baseline_detail else 0
    ideal_sav = (1 - total_ideal / total_baseline_detail) * 100 if total_baseline_detail else 0

    print(f"  {'TOTAL':<12} {'':>8} {'':>8} ${total_baseline_detail:>11.6f} ${total_current_detail:>11.6f} ${total_ideal:>11.6f}")
    print(f"\n  Current router savings:  {cur_sav:.1f}%")
    print(f"  Ideal router savings:    {ideal_sav:.1f}%")
    print(f"  Gap (accuracy cost):     {cur_sav - ideal_sav:+.1f}% (over-routing to cheap models)")

    # --- Phase 6: Diagnosis ---
    print("\n" + "=" * 70)
    print("  Diagnosis")
    print("=" * 70)
    print(f"""
  The keyword-based heuristic scorer uses 10 active dimensions.
  Short prompts with few keywords tend to under-score, while
  prompts with many trigger words can over-score.

  Current tier thresholds (selector.rs):
    SIMPLE   : score <= 0.08
    MEDIUM   : 0.08 < score <= 0.20
    COMPLEX  : 0.20 < score <= 0.30
    REASONING: score > 0.30

  Accuracy: {accuracy:.0f}%  |  Gap vs ideal: {cur_sav - ideal_sav:+.1f}%

  To improve accuracy, consider:
    - Adding more keywords to scorer dimensions
    - Using n-gram or embedding-based scoring
    - Tuning thresholds with a larger labeled dataset
""")
    print()


if __name__ == "__main__":
    run_experiment()
