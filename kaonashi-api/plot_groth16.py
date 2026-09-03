from pathlib import Path
import matplotlib.pyplot as plt

# Measured Criterion medians
batch_sizes = [10, 50, 100]
proof_generation = [5.349884, 34.153702, 100.353728]

output_dir = Path("data/benchmarks/figures")
output_dir.mkdir(parents=True, exist_ok=True)

fig, ax = plt.subplots(figsize=(8, 5))

ax.plot(
    batch_sizes,
    proof_generation,
    marker="o",
    linewidth=2.5,
    markersize=8,
)

ax.set_xlabel("Batch size (votes)")
ax.set_ylabel("Median proof-generation time (s)")
ax.set_xticks(batch_sizes)

ax.grid(axis="y", alpha=0.25)

for x, y in zip(batch_sizes, proof_generation):
    ax.annotate(
        f"{y:.2f} s",
        (x, y),
        xytext=(0, 10),
        textcoords="offset points",
        ha="center",
    )

fig.tight_layout()

fig.savefig(
    output_dir / "groth16_proof_generation.pdf",
    bbox_inches="tight"
)

fig.savefig(
    output_dir / "groth16_proof_generation.png",
    dpi=300,
    bbox_inches="tight"
)

print("Graphs written to:", output_dir)
