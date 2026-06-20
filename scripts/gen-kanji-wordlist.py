#!/usr/bin/env python3
"""Generate the index-aligned kanji "skin" of the BIP-39 Japanese wordlist.

Build-time AID ONLY. Its OUTPUT (crates/core/data/bip39-japanese-kanji.txt) is the
frozen source of truth, committed to the repo. JMdict / KANJIDIC2 (EDRDG, CC-BY-SA)
are used here purely as a lookup aid to determine standard orthography; they are
NOT redistributed. The output is factual orthographic data (the standard kanji
spelling of common words), index-aligned with bitcoin/bips bip-0039/japanese.txt.

Selection: for each BIP-39 reading, among JMdict kanji writings whose *kanji*
characters are all 常用漢字 (kana such as okurigana/suffix is allowed) and which
carry a JMdict frequency/priority marker (ke_pri), pick the most common writing.
The chosen string is just an alphabet symbol for the base-2048 encoder, so a
single canonical, common, well-rendered form per reading is all that is required.
Note: re_restr restrictions are not modelled (acceptable for common words);
the human audit + report is the backstop.
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

PRIMARY = {"news1", "ichi1", "spec1", "gai1"}


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
    """JMdict: reading(kana, NFC) -> {kanji writing (keb): set(ke_pri tags)}."""
    xml = fetch_gz(JMDICT_URL)
    idx: dict = {}
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "entry":
            kebs = []
            for k in el.findall("k_ele"):
                keb = k.findtext("keb")
                if keb:
                    pris = {p.text for p in k.findall("ke_pri") if p.text}
                    kebs.append((keb, pris))
            rebs = [unicodedata.normalize("NFC", kata_to_hira(r.text))
                    for r in el.findall("r_ele/reb") if r.text]
            for r in rebs:
                bucket = idx.setdefault(r, {})
                for keb, pris in kebs:
                    bucket.setdefault(keb, set()).update(pris)
            el.clear()
    return idx


def is_cjk(c: str) -> bool:
    o = ord(c)
    return 0x4E00 <= o <= 0x9FFF or 0x3400 <= o <= 0x4DBF


def is_kana(c: str) -> bool:
    o = ord(c)
    return 0x3041 <= o <= 0x309F or 0x30A1 <= o <= 0x30FF or c == "ー"


def nfc_stable(s: str) -> bool:
    return unicodedata.normalize("NFC", s) == s


def eligible(keb: str, jouyou: set) -> bool:
    """At least one kanji; every kanji char is 常用; other chars are kana; NFC-stable."""
    has_kanji = False
    for c in keb:
        if is_cjk(c):
            if c not in jouyou:
                return False
            has_kanji = True
        elif not is_kana(c):
            return False
    return has_kanji and nfc_stable(keb)


def score(tags: set):
    """Higher is more common: (has primary marker, -smallest nf rank)."""
    primary = 1 if (tags & PRIMARY) else 0
    nfs = [int(t[2:]) for t in tags if t.startswith("nf") and t[2:].isdigit()]
    nf = min(nfs) if nfs else 999
    return (primary, -nf)


def main() -> int:
    words = [w for w in BIP39.read_text(encoding="utf-8").split("\n") if w]
    assert len(words) == 2048, f"expected 2048 BIP-39 words, got {len(words)}"
    jouyou = load_jouyou()
    idx = load_reading_index()
    print(f"jouyou kanji: {len(jouyou)}, reading keys: {len(idx)}", file=sys.stderr)

    result, reasons = [], []
    for w in words:
        key = unicodedata.normalize("NFC", w)
        cands = idx.get(key, {})
        elig = [(keb, tags) for keb, tags in cands.items() if eligible(keb, jouyou)]
        tagged = [(k, t) for k, t in elig if t]
        best, kind = None, "kana:no-eligible"
        if len(elig) == 1:
            # Single standard kanji writing — take it even if JMdict tags it as uncommon
            # (e.g. compounds like 愛国心 that lack frequency markers).
            best, kind = elig[0][0], "kanji"
        elif tagged:
            # Multiple writings: pick the most common one with a frequency marker.
            # This resolves homophones (感謝 over 官舎) by commonness.
            tagged.sort(key=lambda kt: (score(kt[1]), -len(kt[0]), kt[0]), reverse=True)
            best = tagged[0][0]
            alts = [k for k, _ in tagged[1:] if k != best][:3]
            kind = "kanji" + (f"(alt:{'/'.join(alts)})" if alts else "")
        elif elig:
            # Several eligible writings but none carry a frequency marker: likely obscure
            # ateji (e.g. あめりか→亜米利加/亜墨利加). Keep kana — too ambiguous to choose.
            kind = "kana:ambiguous-untagged"
        result.append(best if best is not None else w)
        reasons.append(kind)

    # Collision resolution: a kanji form chosen for >1 position -> revert all to kana.
    counts = Counter(result)
    for i, v in enumerate(result):
        if counts[v] > 1 and v != words[i]:
            result[i] = words[i]
            reasons[i] = "kana:collision"

    assert len(result) == 2048
    assert len(set(result)) == 2048, "post-conversion list is not unique"
    # Chosen kanji forms must be NFC-stable. Kana fallbacks keep BIP-39's NFKD bytes.
    assert all(nfc_stable(result[i]) for i in range(2048) if result[i] != words[i]), \
        "a chosen kanji form is not NFC-stable"

    OUT.write_text("\n".join(result) + "\n", encoding="utf-8")
    REPORT.write_text(
        "\n".join(f"{i}\t{words[i]}\t{reasons[i]}\t{result[i]}" for i in range(2048)) + "\n",
        encoding="utf-8")
    n = sum(1 for r in reasons if r.startswith("kanji"))
    print(f"coverage: {n}/2048 kanji-ified ({100*n/2048:.1f}%)", file=sys.stderr)
    print(f"wrote {OUT} and {REPORT}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
