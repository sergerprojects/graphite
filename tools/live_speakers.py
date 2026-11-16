#!/usr/bin/env python3
"""Live "who is speaking" labels for Graphite, entirely in memory.

Graphite starts this once per recording and sends it each stretch of call audio (the
"Others" side) as it is transcribed. It answers with a speaker number: the same voice
gets the same number for the whole meeting. No audio is ever written to disk.

Protocol (binary on stdin, JSON lines on stdout):
  in : uint32 n_samples, uint32 sample_rate, then n_samples float32 (little endian)
  out: {"ready": true}  once, then per request  {"speaker": <0..>|-1, "sim": <0..1>}
  speaker -1 = too short/unclear to say.

Needs `pip install sherpa-onnx numpy` and the voice model from
`tools/diarize_meeting.py --download-models` (wespeaker_en_voxceleb_resnet34_LM.onnx).
"""
import json
import os
import struct
import sys
from pathlib import Path

import numpy as np

MODELS = Path(os.environ.get("GRAPHITE_SPEAKER_MODELS", Path.home() / ".graphite" / "speaker-models"))
# Best model first (EER 0.45% vs 0.72% for ResNet34, ~5x slower but still ~4% of real time).
MODEL_FILES = ["wespeaker_en_voxceleb_resnet293_LM.onnx", "wespeaker_en_voxceleb_resnet34_LM.onnx"]
PROFILES = Path(os.environ.get("GRAPHITE_VOICE_PROFILES", Path.home() / ".graphite" / "voices.json"))
PROFILE_MATCH = 0.70      # a cluster this close to a saved voice gets that person's name
PROFILE_MARGIN = 0.05     # ...and must beat the next saved voice by this much
TARGET_SR = 16000
MIN_SECONDS = 1.2           # shorter stretches are too unreliable to label
CREATE_MIN_SECONDS = 2.5    # only a longer stretch may start a NEW speaker (short ones split one voice in two)
CENTROID_MIN_SECONDS = 2.0  # only longer stretches move a voice's average
ASSIGN_SIM = float(os.environ.get("GRAPHITE_SPEAKER_SIM", "0.75"))
LOOSE_MARGIN = 0.10         # a shorter stretch may join a voice this much below the strict bar
MAX_SPEAKERS = 12


def max_speakers_from_env(value=None):
    """Cap on distinct voices: the number of other people on the call when known."""
    raw = os.environ.get("GRAPHITE_MAX_SPEAKERS", "") if value is None else value
    try:
        n = int(raw)
    except (TypeError, ValueError):
        return MAX_SPEAKERS
    return max(1, min(n, MAX_SPEAKERS))


class Clusterer:
    """Online clustering of voice embeddings by cosine similarity (pure logic, testable)."""

    def __init__(self, assign_sim=ASSIGN_SIM, max_speakers=MAX_SPEAKERS):
        self.assign_sim = assign_sim
        self.max_speakers = max_speakers
        self.centroids: list[np.ndarray] = []
        self.counts: list[int] = []

    @staticmethod
    def _unit(v):
        v = np.asarray(v, dtype=np.float64)
        n = np.linalg.norm(v)
        return v / n if n > 0 else v

    def assign(self, emb, seconds):
        """Return (speaker, similarity); speaker -1 when the audio is too short to place."""
        e = self._unit(emb)
        best, best_sim = -1, 0.0
        for i, c in enumerate(self.centroids):
            s = float(np.dot(e, c))
            if s > best_sim:
                best, best_sim = i, s
        if seconds < MIN_SECONDS:
            # Very short: only attach to a clearly matching known voice.
            return (best, best_sim) if best_sim >= self.assign_sim else (-1, best_sim)
        if best >= 0 and best_sim >= self.assign_sim:
            if seconds >= CENTROID_MIN_SECONDS:
                n = self.counts[best]
                self.centroids[best] = self._unit(self.centroids[best] * n + e)
                self.counts[best] = n + 1
            return best, best_sim
        if seconds < CREATE_MIN_SECONDS:
            # Not long enough to start a new voice: join the closest one if it is close enough.
            return (best, best_sim) if best >= 0 and best_sim >= self.assign_sim - LOOSE_MARGIN else (-1, best_sim)
        if len(self.centroids) >= self.max_speakers:
            return best, best_sim
        self.centroids.append(e)
        self.counts.append(1)
        return len(self.centroids) - 1, best_sim


def load_profiles(path=PROFILES, model=None):
    """Saved voices: {name: [unit vectors]}. Ignored when made with a different model."""
    try:
        data = json.loads(Path(path).read_text())
    except Exception:
        return {}
    if model and data.get("model") != model:
        return {}
    out = {}
    for name, vecs in (data.get("people") or {}).items():
        arr = [np.asarray(v, dtype=np.float64) for v in vecs]
        arr = [v / np.linalg.norm(v) for v in arr if v.size and np.linalg.norm(v) > 0]
        if arr:
            out[name] = arr
    return out


def match_profile(centroid, profiles, match=PROFILE_MATCH, margin=PROFILE_MARGIN):
    """Name of the saved voice this centroid is clearly closest to, else None."""
    c = np.asarray(centroid, dtype=np.float64)
    n = np.linalg.norm(c)
    if n == 0 or not profiles:
        return None
    c = c / n
    best = sorted(((max(float(np.dot(c, v)) for v in vecs), name) for name, vecs in profiles.items()), reverse=True)
    if best[0][0] < match:
        return None
    if len(best) > 1 and best[0][0] - best[1][0] < margin:
        return None
    return best[0][1]


def resample(x, sr):
    if sr == TARGET_SR or len(x) == 0:
        return x
    n = int(len(x) * TARGET_SR / sr)
    return np.interp(np.linspace(0, len(x) - 1, n), np.arange(len(x)), x).astype(np.float32)


def main():
    import sherpa_onnx
    model = next((MODELS / f for f in MODEL_FILES if (MODELS / f).exists()), None)
    if model is None:
        print(json.dumps({"error": f"voice model missing in {MODELS}"}), flush=True)
        return 2
    cfg = sherpa_onnx.SpeakerEmbeddingExtractorConfig(model=str(model), num_threads=2)
    extractor = sherpa_onnx.SpeakerEmbeddingExtractor(cfg)
    clusterer = Clusterer(max_speakers=max_speakers_from_env())
    profiles = load_profiles(model=model.name)
    print(json.dumps({"ready": True, "model": model.name, "profiles": len(profiles)}), flush=True)
    stdin = sys.stdin.buffer
    while True:
        head = stdin.read(8)
        if len(head) < 8:
            return 0
        n, sr = struct.unpack("<II", head)
        raw = stdin.read(n * 4)
        if len(raw) < n * 4:
            return 0
        samples = resample(np.frombuffer(raw, dtype=np.float32), sr)
        seconds = len(samples) / TARGET_SR
        if seconds < 0.6:
            print(json.dumps({"speaker": -1, "sim": 0.0}), flush=True)
            continue
        stream = extractor.create_stream()
        stream.accept_waveform(sample_rate=TARGET_SR, waveform=samples)
        stream.input_finished()
        emb = np.array(extractor.compute(stream), dtype=np.float32)
        speaker, sim = clusterer.assign(emb, seconds)
        reply = {"speaker": int(speaker), "sim": round(float(sim), 3), "sec": round(seconds, 2)}
        if seconds >= CENTROID_MIN_SECONDS:
            # Kept (as unit vectors) in the meeting's voice journal for the end-of-meeting pass.
            u = emb / (np.linalg.norm(emb) or 1.0)
            reply["emb"] = [round(float(x), 4) for x in u]
        if speaker >= 0 and profiles:
            name = match_profile(clusterer.centroids[speaker], profiles)
            if name:
                reply["name"] = name
        print(json.dumps(reply), flush=True)


if __name__ == "__main__":
    sys.exit(main())
