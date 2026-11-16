#!/usr/bin/env bash
# Optional: enable live "who is speaking" labels (Speaker 1, 2, ...) for the call side.
# Creates ~/.graphite/venv with sherpa-onnx and downloads the voice models (~135 MB, from the
# k2-fsa/sherpa-onnx GitHub releases): the accurate WeSpeaker ResNet293 for live labels, and
# the small ResNet34 + pyannote segmentation for the offline tool. Audio is analysed in memory only and never saved.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
command -v python3 >/dev/null || { echo "python3 is required"; exit 1; }
[ -x "$HOME/.graphite/venv/bin/python" ] || python3 -m venv "$HOME/.graphite/venv"
"$HOME/.graphite/venv/bin/pip" install -q sherpa-onnx numpy
"$HOME/.graphite/venv/bin/python" "$ROOT/tools/diarize_meeting.py" --download-models
BEST="$HOME/.graphite/speaker-models/wespeaker_en_voxceleb_resnet293_LM.onnx"
[ -s "$BEST" ] || curl -fL --progress-bar -o "$BEST" "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet293_LM.onnx"
echo "Live speaker labels are ready. Restart Graphite; they start with the next recording."
