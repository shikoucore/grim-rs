# Profiling Manual (grim-rs)

Purpose: collect **performance** (Criterion) and **memory** (DHAT, heaptrack, Massif) data in a Wayland session.

## Requirements

- Run inside a Wayland session.
- Run commands from the repository root.
- Tools: `valgrind`, `ms_print`, `heaptrack`, `heaptrack_print`.

## Quick start (scripts)

**Full dataset (Criterion + DHAT + heaptrack + Massif):**

```bash
./scripts/run_performance.sh
```

Behavior:
- Prompts to delete old outputs (use `--clean` or `--no-clean`).
- Runs Criterion, DHAT, heaptrack, Massif in the order below.

**Fair comparison (reproducible speed + memory summary):**

```bash
./scripts/run_fair_compare.sh --runs=3
```

Outputs (summary only):
- `target/fair_compare/summary_encode.md`
- `target/fair_compare/summary_capture.md`
- `target/fair_compare/summary_encode_compare.md`
- `target/fair_compare/summary_capture_compare.md`
- `target/fair_compare/summary_memory.md`
- `target/fair_compare/summary_memory_compare.md`

## Recommended sequence (full dataset)

1) **Clean old outputs (optional, but recommended)**

```bash
rm -rf target/dhat target/heaptrack_*.txt target/heaptrack_*.heap.zst target/massif_*.out target/massif_*.txt target/criterion target/bench_*.txt
```

2) **Build bench binaries**

```bash
cargo bench --bench encode_benchmarks --no-run
cargo bench --bench capture_benchmarks --no-run

ENC=$(ls target/release/deps/encode_benchmarks-* | head -n1)
CAP=$(ls target/release/deps/capture_benchmarks-* | head -n1)

test -x "$ENC" && test -x "$CAP" || { echo "bench binaries not found"; exit 1; }
```

3) **Performance (Criterion)**

```bash
cargo bench --bench encode_benchmarks -- --noplot > target/bench_encode.txt
cargo bench --bench capture_benchmarks -- --noplot > target/bench_capture.txt
```

4) **DHAT (alloc_profiler)**

```bash
rm -rf target/dhat
cargo bench --bench alloc_profiler
```

5) **heaptrack (peak + allocation stacks)**

```bash
heaptrack --output target/heaptrack_encode.heap "$ENC" --sample-size 10 --measurement-time 1
heaptrack_print target/heaptrack_encode.heap.zst > target/heaptrack_encode.txt

heaptrack --output target/heaptrack_capture.heap "$CAP" --sample-size 10 --measurement-time 1
heaptrack_print target/heaptrack_capture.heap.zst > target/heaptrack_capture.txt
```

6) **Massif (peak over time)**

```bash
valgrind --tool=massif --time-unit=B --massif-out-file=target/massif_encode.out "$ENC" --sample-size 10 --measurement-time 1
ms_print target/massif_encode.out > target/massif_encode.txt

valgrind --tool=massif --time-unit=B --massif-out-file=target/massif_capture.out "$CAP" --sample-size 10 --measurement-time 1
ms_print target/massif_capture.out > target/massif_capture.txt
```

## Final dataset to collect

- `target/dhat/*.json`
- `target/heaptrack_encode.txt`
- `target/heaptrack_capture.txt`
- `target/massif_encode.txt`
- `target/massif_capture.txt`
- `target/bench_encode.txt`
- `target/bench_capture.txt`
