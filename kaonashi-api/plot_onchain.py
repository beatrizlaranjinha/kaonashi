import csv
import statistics
from collections import defaultdict
from pathlib import Path

import matplotlib.pyplot as plt


INPUT = "data/benchmarks/onchain.csv"
OUTPUT_DIR = Path("data/benchmarks/figures")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)


# ============================================================
# Load data
# ============================================================

groups = defaultdict(list)

with open(INPUT, newline="") as f:
    reader = csv.DictReader(f)

    for row in reader:
        batch = int(row["batch_size"])

        groups[batch].append({
            "latency": float(row["send_to_confirmed_latency_ms"]),
            "cu": float(row["compute_units"]),
            "fee": float(row["fee_lamports"]),
            "throughput": float(row["throughput_votes_s"]),
            "cu_per_vote": float(row["cu_per_vote"]),
            "fee_per_vote": float(row["fee_per_vote_lamports"]),
        })


batches = sorted(groups.keys())


def mean(metric):
    return [
        statistics.mean(x[metric] for x in groups[b])
        for b in batches
    ]


def median(metric):
    return [
        statistics.median(x[metric] for x in groups[b])
        for b in batches
    ]


def std(metric):
    return [
        statistics.stdev(x[metric] for x in groups[b])
        for b in batches
    ]


# ============================================================
# 1. On-chain latency
# ============================================================

values = median("latency")

plt.figure(figsize=(6.5, 4.2))
plt.plot(batches, values, marker="o", linewidth=2)

plt.xlabel("Batch size (votes)")
plt.ylabel("Median send-to-confirmed latency (ms)")
plt.xticks(batches)
plt.grid(axis="y", alpha=0.25)
plt.tight_layout()

plt.savefig(
    OUTPUT_DIR / "onchain_latency.pdf",
    bbox_inches="tight"
)
plt.savefig(
    OUTPUT_DIR / "onchain_latency.png",
    dpi=300,
    bbox_inches="tight"
)

plt.close()


# ============================================================
# 2. Compute Units per vote
# ============================================================

values = mean("cu_per_vote")

plt.figure(figsize=(6.5, 4.2))
plt.plot(batches, values, marker="o", linewidth=2)

plt.xlabel("Batch size (votes)")
plt.ylabel("Compute units per vote")
plt.xticks(batches)
plt.grid(axis="y", alpha=0.25)
plt.tight_layout()

plt.savefig(
    OUTPUT_DIR / "cu_per_vote.pdf",
    bbox_inches="tight"
)
plt.savefig(
    OUTPUT_DIR / "cu_per_vote.png",
    dpi=300,
    bbox_inches="tight"
)

plt.close()


# ============================================================
# 3. Effective throughput
# ============================================================

values = median("throughput")

plt.figure(figsize=(6.5, 4.2))
plt.plot(batches, values, marker="o", linewidth=2)

plt.xlabel("Batch size (votes)")
plt.ylabel("Median effective throughput (votes/s)")
plt.xticks(batches)
plt.grid(axis="y", alpha=0.25)
plt.tight_layout()

plt.savefig(
    OUTPUT_DIR / "effective_throughput.pdf",
    bbox_inches="tight"
)
plt.savefig(
    OUTPUT_DIR / "effective_throughput.png",
    dpi=300,
    bbox_inches="tight"
)

plt.close()


# ============================================================
# 4. Total Compute Units
# ============================================================

values = mean("cu")
errors = std("cu")

plt.figure(figsize=(6.5, 4.2))
plt.errorbar(
    batches,
    values,
    yerr=errors,
    marker="o",
    linewidth=2,
    capsize=4,
)

plt.xlabel("Batch size (votes)")
plt.ylabel("Compute units per rollup transaction")
plt.xticks(batches)
plt.grid(axis="y", alpha=0.25)
plt.tight_layout()

plt.savefig(
    OUTPUT_DIR / "total_compute_units.pdf",
    bbox_inches="tight"
)
plt.savefig(
    OUTPUT_DIR / "total_compute_units.png",
    dpi=300,
    bbox_inches="tight"
)

plt.close()


# ============================================================
# 5. Fee per vote
# ============================================================

values = mean("fee_per_vote")

plt.figure(figsize=(6.5, 4.2))
plt.plot(batches, values, marker="o", linewidth=2)

plt.xlabel("Batch size (votes)")
plt.ylabel("Transaction fee per vote (lamports)")
plt.xticks(batches)
plt.grid(axis="y", alpha=0.25)
plt.tight_layout()

plt.savefig(
    OUTPUT_DIR / "fee_per_vote.pdf",
    bbox_inches="tight"
)
plt.savefig(
    OUTPUT_DIR / "fee_per_vote.png",
    dpi=300,
    bbox_inches="tight"
)

plt.close()


print(f"Graphs written to: {OUTPUT_DIR}")
