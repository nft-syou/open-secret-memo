#!/usr/bin/env python3
"""Generate the index-aligned kanji "skin" of the BIP-39 Japanese wordlist.

Build-time AID ONLY. Its OUTPUT (crates/core/data/bip39-japanese-kanji.txt) is the
frozen source of truth, committed to the repo. JMdict / KANJIDIC2 (EDRDG, CC-BY-SA)
are used here purely as a lookup aid to determine standard orthography; they are
NOT redistributed. The output is factual orthographic data (the standard kanji
spelling of common words), index-aligned with bitcoin/bips bip-0039/japanese.txt.
"""
import gzip, io, sys, unicodedata, urllib.request
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

BIP39 = Path("crates/core/data/bip39-japanese.txt")
OUT = Path("crates/core/data/bip39-japanese-kanji.txt")
REPORT = Path("scripts/kanji-wordlist-report.tsv")
JMDICT_URL = "http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz"
KANJIDIC_URL = "http://ftp.edrdg.org/pub/Nihongo/kanjidic2.xml.gz"


def kata_to_hira(s: str) -> str:
    return "".join(chr(ord(c) - 0x60) if 0x30A1 <= ord(c) <= 0x30F6 else c for c in s)


def fetch_gz(url: str) -> bytes:
    print(f"fetching {url} ...", file=sys.stderr)
    with urllib.request.urlopen(url, timeout=120) as r:
        return gzip.decompress(r.read())


def load_jouyou() -> set:
    """KANJIDIC2: characters with misc/grade 1..8 are 常用漢字 (9,10 = 人名用)."""
    xml = fetch_gz(KANJIDIC_URL)
    jouyou = set()
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "character":
            lit = el.findtext("literal")
            grade = el.findtext("misc/grade")
            if lit and grade and grade.isdigit() and 1 <= int(grade) <= 8:
                jouyou.add(lit)
            el.clear()
    return jouyou


def load_reading_index() -> dict:
    """JMdict: reading(kana, hira-normalized) -> set of kanji writings (keb)."""
    xml = fetch_gz(JMDICT_URL)
    idx = {}
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "entry":
            kebs = [k.text for k in el.findall("k_ele/keb") if k.text]
            # Normalize readings to NFC for matching against NFC-normalized BIP-39 words.
            rebs = [unicodedata.normalize("NFC", kata_to_hira(r.text))
                    for r in el.findall("r_ele/reb") if r.text]
            for r in rebs:
                idx.setdefault(r, set()).update(kebs)
            el.clear()
    return idx


def is_pure_jouyou(s: str, jouyou: set) -> bool:
    return bool(s) and all(ch in jouyou for ch in s)


def nfkc_stable(s: str) -> bool:
    return unicodedata.normalize("NFKC", s) == s


def main() -> int:
    words = [w for w in BIP39.read_text(encoding="utf-8").split("\n") if w]
    assert len(words) == 2048, f"expected 2048 BIP-39 words, got {len(words)}"
    jouyou = load_jouyou()
    idx = load_reading_index()
    print(f"jouyou kanji: {len(jouyou)}, reading keys: {len(idx)}", file=sys.stderr)

    result, reasons = [], []
    for w in words:
        # BIP-39 JA words are NFKD; match against NFC reading keys. Kana fallback
        # below stores the ORIGINAL w (NFKD) to stay byte-identical to bip39-japanese.txt.
        key = unicodedata.normalize("NFC", w)
        valid = sorted({k for k in idx.get(key, set())
                        if is_pure_jouyou(k, jouyou) and nfkc_stable(k)})
        if len(valid) == 1:
            result.append(valid[0]); reasons.append("kanji")
        elif not valid:
            result.append(w); reasons.append("kana:no-candidate")
        else:
            result.append(w); reasons.append("kana:ambiguous(" + "/".join(valid) + ")")

    # Collision resolution: a kanji form chosen for >1 position -> revert all to kana.
    counts = Counter(result)
    for i, v in enumerate(result):
        if counts[v] > 1 and v != words[i]:
            result[i] = words[i]; reasons[i] = "kana:collision"

    assert len(result) == 2048
    assert len(set(result)) == 2048, "post-conversion list is not unique"
    # Chosen kanji forms must be NFC-stable (atomic ideographs). Kana fallbacks keep
    # BIP-39's canonical NFKD bytes for index alignment, so they are exempt.
    assert all(nfkc_stable(result[i]) for i in range(2048) if result[i] != words[i]), \
        "a chosen kanji form is not NFC-stable"

    OUT.write_text("\n".join(result) + "\n", encoding="utf-8")
    REPORT.write_text(
        "\n".join(f"{i}\t{words[i]}\t{reasons[i]}\t{result[i]}" for i in range(2048)) + "\n",
        encoding="utf-8")
    n = sum(1 for r in reasons if r == "kanji")
    print(f"coverage: {n}/2048 kanji-ified ({100*n/2048:.1f}%)", file=sys.stderr)
    print(f"wrote {OUT} and {REPORT}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
