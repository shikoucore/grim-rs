#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

log() {
  printf '[%s] %s\n' "$(date +'%H:%M:%S')" "$*"
}

if [[ -z "${WAYLAND_DISPLAY:-}" || -z "${XDG_RUNTIME_DIR:-}" ]]; then
  echo "WAYLAND_DISPLAY or XDG_RUNTIME_DIR is not set. Run inside a Wayland session." >&2
  exit 1
fi
if [[ ! -S "${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY}" ]]; then
  echo "Wayland socket not found at ${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY}" >&2
  exit 1
fi

RUNS=3
CLEAN=1

for arg in "$@"; do
  case "$arg" in
    --runs=*)
      RUNS="${arg#*=}"
      ;;
    --no-clean)
      CLEAN=0
      ;;
    --clean)
      CLEAN=1
      ;;
    -h|--help)
      echo "Usage: $0 [--runs=N] [--clean|--no-clean]"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || [[ "$RUNS" -lt 1 ]]; then
  echo "Invalid --runs value: $RUNS" >&2
  exit 1
fi

OUT_DIR="target/fair_compare"

if [[ "$CLEAN" -eq 1 ]]; then
  log "Cleaning $OUT_DIR"
  rm -rf "$OUT_DIR"
fi

mkdir -p "$OUT_DIR"

log "Running fair comparison: $RUNS runs"

for i in $(seq 1 "$RUNS"); do
  log "Run $i/$RUNS: encode_benchmarks"
  cargo bench --bench encode_benchmarks -- --noplot > "$OUT_DIR/bench_encode_run${i}.txt"
  log "Run $i/$RUNS: capture_benchmarks"
  cargo bench --bench capture_benchmarks -- --noplot > "$OUT_DIR/bench_capture_run${i}.txt"
done

python - <<'PY'
import re
from pathlib import Path
import statistics as stats

root = Path("target/fair_compare")
baseline_path = Path("doc/performance.md")
benchmarks_path = Path("doc/benchmarks.md")

def parse_bench(path):
    rows = {}
    cur = None
    for line in path.read_text().splitlines():
        m = re.match(r"^([A-Za-z0-9_./]+)\s+time:\s+\[([0-9.]+)\s+([a-zµ]+)\s+([0-9.]+)\s+([a-zµ]+)\s+([0-9.]+)\s+([a-zµ]+)\]", line)
        if m:
            name = m.group(1)
            mid = float(m.group(4))
            unit = m.group(5)
            rows[name] = (mid, unit)
            continue
        m2 = re.match(r"^([A-Za-z0-9_./]+)\s*$", line)
        if m2:
            cur = m2.group(1)
            continue
        m3 = re.match(r"^\s*time:\s+\[([0-9.]+)\s+([a-zµ]+)\s+([0-9.]+)\s+([a-zµ]+)\s+([0-9.]+)\s+([a-zµ]+)\]", line)
        if m3 and cur:
            mid = float(m3.group(3))
            unit = m3.group(4)
            rows[cur] = (mid, unit)
            cur = None
            continue
    return rows

def to_ms(val, unit):
    return val * {'ns':1e-6,'us':1e-3,'µs':1e-3,'ms':1,'s':1000}.get(unit,1)

def parse_baseline(path):
    rows = {}
    if not path.exists():
        return rows
    for line in path.read_text().splitlines():
        if not line.startswith('| `'):
            continue
        parts = [p.strip() for p in line.split('|')]
        if len(parts) < 5:
            continue
        group = parts[1].strip('`')
        case = parts[2].strip('`')
        try:
            median_s = float(parts[4])
        except ValueError:
            continue
        rows[f"{group}/{case}"] = median_s * 1000.0
    return rows

def parse_benchmarks_baseline(path):
    dhat = {}
    massif = {}
    heaptrack = {}
    if not path.exists():
        return dhat, massif, heaptrack
    section = None
    for line in path.read_text().splitlines():
        if line.startswith('## DHAT'):
            section = 'dhat'
            continue
        if line.startswith('## Massif'):
            section = 'massif'
            continue
        if line.startswith('## heaptrack'):
            section = 'heaptrack'
            continue
        if not line.startswith('|'):
            continue
        parts = [p.strip() for p in line.split('|')]
        if len(parts) < 4 or parts[1].startswith('---'):
            continue
        if section == 'dhat':
            case = parts[1].strip('`')
            try:
                peak = int(parts[2].replace(',', ''))
            except ValueError:
                continue
            dhat[case] = peak
        elif section == 'massif':
            case = parts[1].strip('`')
            try:
                peak = int(parts[2].replace(',', ''))
            except ValueError:
                continue
            massif[case] = peak
        elif section == 'heaptrack':
            case = parts[1].strip('`')
            try:
                peak = int(parts[2].replace(',', ''))
            except ValueError:
                continue
            heaptrack[case] = peak
    return dhat, massif, heaptrack

def dhat_peak_bytes(path):
    try:
        import json
        d = json.loads(path.read_text())
        vals = [pp.get('tb', 0) for pp in d.get('pps', [])]
        return max(vals) if vals else 0
    except Exception:
        return 0

def massif_peak_total(path):
    if not path.exists():
        return None
    rows = []
    for line in path.read_text().splitlines():
        m = re.match(r"\\s*\\d+\\s+([0-9,]+)\\s+([0-9,]+)\\s+([0-9,]+)\\s+([0-9,]+)\\s+([0-9,]+)", line)
        if not m:
            continue
        total_b = int(m.group(2).replace(',', ''))
        rows.append(total_b)
    return max(rows) if rows else None

def heaptrack_peak(path):
    if not path.exists():
        return None
    for line in path.read_text().splitlines():
        if line.startswith('peak heap memory consumption:'):
            val = line.split(':', 1)[1].strip()
            m = re.match(r"([0-9.]+)([KMG]?)", val)
            if not m:
                return None
            num = float(m.group(1))
            unit = m.group(2)
            mult = {'':1, 'K':1024, 'M':1024**2, 'G':1024**3}.get(unit, 1)
            return int(num * mult)
    return None

def collect(prefix):
    runs = sorted(root.glob(f"{prefix}_run*.txt"))
    data = {}
    for r in runs:
        rows = parse_bench(r)
        for name, (mid, unit) in rows.items():
            data.setdefault(name, []).append(to_ms(mid, unit))
    return data, runs

enc, enc_runs = collect("bench_encode")
cap, cap_runs = collect("bench_capture")

def write_summary(name, data):
    lines = [
        f"# Fair Comparison Summary: {name}",
        "",
        "Values are **median of medians** across runs (ms).",
        "",
        "| Case | Runs | Median (ms) | Min (ms) | Max (ms) |",
        "|---|---:|---:|---:|---:|",
    ]
    for key in sorted(data.keys()):
        vals = data[key]
        if not vals:
            continue
        med = stats.median(vals)
        lines.append(f"| `{key}` | {len(vals)} | {med:.3f} | {min(vals):.3f} | {max(vals):.3f} |")
    out = root / f"summary_{name}.md"
    out.write_text("\n".join(lines) + "\n")
    print("Wrote", out)

write_summary("encode", enc)
write_summary("capture", cap)

baseline = parse_baseline(baseline_path)

def write_compare(name, data):
    lines = [
        f"# Fair Comparison vs Baseline: {name}",
        "",
        "Values are **median of medians** across runs (ms). Baseline is `doc/performance.md`.",
        "",
        "| Case | Runs | Baseline (ms) | Current (ms) | Δ% | Min (ms) | Max (ms) |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for key in sorted(data.keys()):
        vals = data[key]
        if not vals:
            continue
        med = stats.median(vals)
        base = baseline.get(key)
        if base is None:
            base_s = "n/a"
            delta = "n/a"
        else:
            base_s = f"{base:.3f}"
            delta = f"{((med - base) / base * 100.0):+.1f}"
        lines.append(
            f"| `{key}` | {len(vals)} | {base_s} | {med:.3f} | {delta} | {min(vals):.3f} | {max(vals):.3f} |"
        )
    out = root / f"summary_{name}_compare.md"
    out.write_text("\n".join(lines) + "\n")
    print("Wrote", out)

write_compare("encode", enc)
write_compare("capture", cap)

# Memory summaries (DHAT/Massif/heaptrack)
dhat_base, massif_base, heaptrack_base = parse_benchmarks_baseline(benchmarks_path)

def write_memory_summary():
    lines = [
        "# Fair Comparison Summary: Memory",
        "",
        "Current values are extracted from the latest profiling outputs in `target/`.",
        "",
        "## DHAT (peak bytes)",
        "",
        "| Case | Peak bytes |",
        "|---|---:|",
    ]
    dhat_dir = Path("target/dhat")
    if dhat_dir.exists():
        for p in sorted(dhat_dir.glob("*.json")):
            case = p.stem
            peak = dhat_peak_bytes(p)
            lines.append(f"| `{case}` | {peak:,} |")
    else:
        lines.append("| (missing) | n/a |")

    lines += [
        "",
        "## Massif (peak heap)",
        "",
        "| Bench | Peak bytes |",
        "|---|---:|",
    ]
    enc_peak = massif_peak_total(Path('target/massif_encode.txt'))
    cap_peak = massif_peak_total(Path('target/massif_capture.txt'))
    lines.append(f"| `encode_benchmarks` | {enc_peak:,} |" if enc_peak is not None else "| `encode_benchmarks` | n/a |")
    lines.append(f"| `capture_benchmarks` | {cap_peak:,} |" if cap_peak is not None else "| `capture_benchmarks` | n/a |")

    lines += [
        "",
        "## heaptrack (peak heap)",
        "",
        "| Bench | Peak bytes |",
        "|---|---:|",
    ]
    enc_heap = heaptrack_peak(Path('target/heaptrack_encode.txt'))
    cap_heap = heaptrack_peak(Path('target/heaptrack_capture.txt'))
    lines.append(f"| `encode_benchmarks` | {enc_heap:,} |" if enc_heap is not None else "| `encode_benchmarks` | n/a |")
    lines.append(f"| `capture_benchmarks` | {cap_heap:,} |" if cap_heap is not None else "| `capture_benchmarks` | n/a |")

    out = root / "summary_memory.md"
    out.write_text("\n".join(lines) + "\n")
    print("Wrote", out)

def write_memory_compare():
    lines = [
        "# Fair Comparison vs Baseline: Memory",
        "",
        "Baseline is `doc/benchmarks.md`.",
        "",
        "## DHAT (peak bytes)",
        "",
        "| Case | Baseline | Current | Δ% |",
        "|---|---:|---:|---:|",
    ]
    dhat_dir = Path("target/dhat")
    if dhat_dir.exists():
        cases = sorted(set(dhat_base.keys()) | {p.stem for p in dhat_dir.glob('*.json')})
    else:
        cases = sorted(set(dhat_base.keys()))
    for case in cases:
        base = dhat_base.get(case)
        cur = dhat_peak_bytes(dhat_dir / f"{case}.json") if dhat_dir.exists() else 0
        if base is None or base == 0:
            lines.append(f"| `{case}` | n/a | {cur:,} | n/a |")
        else:
            delta = (cur - base) / base * 100.0
            lines.append(f"| `{case}` | {base:,} | {cur:,} | {delta:+.1f}% |")

    lines += [
        "",
        "## Massif (peak heap)",
        "",
        "| Bench | Baseline | Current | Δ% |",
        "|---|---:|---:|---:|",
    ]
    cur_enc = massif_peak_total(Path('target/massif_encode.txt'))
    cur_cap = massif_peak_total(Path('target/massif_capture.txt'))
    base_enc = massif_base.get('encode_benchmarks', 0)
    base_cap = massif_base.get('capture_benchmarks', 0)
    if base_enc and cur_enc is not None:
        lines.append(f"| `encode_benchmarks` | {base_enc:,} | {cur_enc:,} | {((cur_enc-base_enc)/base_enc*100.0):+.2f}% |")
    else:
        cur_str = f"{cur_enc:,}" if cur_enc is not None else "n/a"
        lines.append(f"| `encode_benchmarks` | n/a | {cur_str} | n/a |")
    if base_cap and cur_cap is not None:
        lines.append(f"| `capture_benchmarks` | {base_cap:,} | {cur_cap:,} | {((cur_cap-base_cap)/base_cap*100.0):+.2f}% |")
    else:
        cur_str = f"{cur_cap:,}" if cur_cap is not None else "n/a"
        lines.append(f"| `capture_benchmarks` | n/a | {cur_str} | n/a |")

    lines += [
        "",
        "## heaptrack (peak heap)",
        "",
        "| Bench | Baseline | Current | Δ% |",
        "|---|---:|---:|---:|",
    ]
    cur_henc = heaptrack_peak(Path('target/heaptrack_encode.txt'))
    cur_hcap = heaptrack_peak(Path('target/heaptrack_capture.txt'))
    base_henc = heaptrack_base.get('encode_benchmarks', 0)
    base_hcap = heaptrack_base.get('capture_benchmarks', 0)
    if base_henc and cur_henc is not None:
        lines.append(f"| `encode_benchmarks` | {base_henc:,} | {cur_henc:,} | {((cur_henc-base_henc)/base_henc*100.0):+.2f}% |")
    else:
        cur_str = f"{cur_henc:,}" if cur_henc is not None else "n/a"
        lines.append(f"| `encode_benchmarks` | n/a | {cur_str} | n/a |")
    if base_hcap and cur_hcap is not None:
        lines.append(f"| `capture_benchmarks` | {base_hcap:,} | {cur_hcap:,} | {((cur_hcap-base_hcap)/base_hcap*100.0):+.2f}% |")
    else:
        cur_str = f"{cur_hcap:,}" if cur_hcap is not None else "n/a"
        lines.append(f"| `capture_benchmarks` | n/a | {cur_str} | n/a |")

    out = root / "summary_memory_compare.md"
    out.write_text("\n".join(lines) + "\n")
    print("Wrote", out)

write_memory_summary()
write_memory_compare()
PY

log "Wrote summaries:"
log "  $OUT_DIR/summary_encode.md"
log "  $OUT_DIR/summary_capture.md"
log "  $OUT_DIR/summary_encode_compare.md"
log "  $OUT_DIR/summary_capture_compare.md"
log "  $OUT_DIR/summary_memory.md"
log "  $OUT_DIR/summary_memory_compare.md"
