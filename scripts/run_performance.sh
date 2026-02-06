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

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required tool: $1" >&2
    exit 1
  fi
}

require_cmd valgrind
require_cmd ms_print
require_cmd heaptrack
require_cmd heaptrack_print

DISCUSS_CLEAN="prompt"
CLEAN=0
RUN_DHAT=1
for arg in "$@"; do
  case "$arg" in
    --no-clean) DISCUSS_CLEAN=0; CLEAN=0 ;;
    --clean) DISCUSS_CLEAN=0; CLEAN=1 ;;
    --dhat) RUN_DHAT=1 ;;
    --no-dhat) RUN_DHAT=0 ;;
    -h|--help)
      echo "Usage: $0 [--clean|--no-clean] [--dhat|--no-dhat]"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

if [[ "$DISCUSS_CLEAN" == "prompt" ]]; then
  if [[ -t 0 ]]; then
    read -r -p "Delete previous benchmark/profiling data? [y/N] " reply
    case "${reply,,}" in
      y|yes) CLEAN=1 ;;
      *) CLEAN=0 ;;
    esac
  else
    CLEAN=0
  fi
fi

if [[ "$CLEAN" -eq 1 ]]; then
  log "Cleaning previous outputs"
  rm -rf target/criterion target/dhat \
    target/bench_encode.txt target/bench_capture.txt \
    target/massif_encode.out target/massif_encode.txt \
    target/massif_capture.out target/massif_capture.txt \
    target/heaptrack_encode.heap target/heaptrack_encode.heap.zst target/heaptrack_encode.txt \
    target/heaptrack_capture.heap target/heaptrack_capture.heap.zst target/heaptrack_capture.txt
else
  log "Keeping existing outputs"
fi

log "Running Criterion encode benchmarks"
cargo bench --bench encode_benchmarks -- --noplot > target/bench_encode.txt

log "Running Criterion capture benchmarks"
cargo bench --bench capture_benchmarks -- --noplot > target/bench_capture.txt

if [[ "$RUN_DHAT" -eq 1 ]]; then
  log "Running DHAT alloc_profiler"
  rm -rf target/dhat
  cargo bench --bench alloc_profiler
fi

log "Building bench binaries"
cargo bench --bench encode_benchmarks --no-run
cargo bench --bench capture_benchmarks --no-run

ENC="$(find target/release/deps -maxdepth 1 -type f -executable -name 'encode_benchmarks-*' | head -n1)"
CAP="$(find target/release/deps -maxdepth 1 -type f -executable -name 'capture_benchmarks-*' | head -n1)"
if [[ -z "$ENC" || -z "$CAP" ]]; then
  echo "bench binaries not found in target/release/deps" >&2
  exit 1
fi

MASSIF_RAN=0
HEAPTRACK_RAN=0

log "Running Massif (encode)"
valgrind --tool=massif --time-unit=B --massif-out-file=target/massif_encode.out "$ENC" --sample-size 10 --measurement-time 1
ms_print target/massif_encode.out > target/massif_encode.txt

log "Running Massif (capture)"
valgrind --tool=massif --time-unit=B --massif-out-file=target/massif_capture.out "$CAP" --sample-size 10 --measurement-time 1
ms_print target/massif_capture.out > target/massif_capture.txt
MASSIF_RAN=1

log "Running heaptrack (encode)"
heaptrack --output target/heaptrack_encode.heap "$ENC" --sample-size 10 --measurement-time 1
heaptrack_print target/heaptrack_encode.heap.zst > target/heaptrack_encode.txt

log "Running heaptrack (capture)"
heaptrack --output target/heaptrack_capture.heap "$CAP" --sample-size 10 --measurement-time 1
heaptrack_print target/heaptrack_capture.heap.zst > target/heaptrack_capture.txt
HEAPTRACK_RAN=1

echo "Wrote:"
echo "  target/bench_encode.txt"
echo "  target/bench_capture.txt"
echo "Criterion data in target/criterion/"
echo "  target/massif_encode.txt"
echo "  target/massif_capture.txt"
echo "  target/heaptrack_encode.txt"
echo "  target/heaptrack_capture.txt"
if [[ "$RUN_DHAT" -eq 1 ]]; then
  echo "  target/dhat/*.json"
fi

echo "Ran:"
echo "  Criterion benches: yes"
echo "  DHAT: $( [[ $RUN_DHAT -eq 1 ]] && echo yes || echo no )"
echo "  Massif: $( [[ $MASSIF_RAN -eq 1 ]] && echo yes || echo no )"
echo "  Heaptrack: $( [[ $HEAPTRACK_RAN -eq 1 ]] && echo yes || echo no )"
