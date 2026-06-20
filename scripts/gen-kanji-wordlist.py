#!/usr/bin/env python3
"""Generate the index-aligned kanji/kana "skin" of the BIP-39 Japanese wordlist.

Build-time AID ONLY. Its OUTPUT (crates/core/data/bip39-japanese-kanji.txt) is the
frozen source of truth, committed to the repo. JMdict / JMnedict / KANJIDIC2 (EDRDG,
CC-BY-SA) are used here purely as lookup aids to determine standard orthography; they
are NOT redistributed. The output is factual orthographic data, index-aligned with
bitcoin/bips bip-0039/japanese.txt.

Per BIP-39 reading, choose the most natural standard written form:
  1. kanji      — JMdict kanji writing whose kanji chars are all 常用漢字 (kana such as
                  okurigana allowed). Single candidate taken as-is (愛国心); multiple
                  resolved by JMdict frequency markers (感謝 over 官舎).
  2. katakana   — loanwords/foreign words (JMdict stores the reading in katakana):
                  あめりか→アメリカ, たいみんぐ→タイミング.
  3. kanji-name — proper nouns via JMnedict (place/person names): かなざわし→金沢市.
  4. hiragana   — otherwise keep the original BIP-39 hiragana.
The chosen string is just an alphabet symbol for the base-2048 encoder.
"""
import gzip, io, sys, unicodedata, urllib.request
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

BIP39 = Path("crates/core/data/bip39-japanese.txt")
OUT = Path("crates/core/data/bip39-japanese-kanji.txt")
REPORT = Path("scripts/kanji-wordlist-report.tsv")
JMDICT_URL = "http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz"
JMNEDICT_URL = "http://ftp.edrdg.org/pub/Nihongo/JMnedict.xml.gz"
KANJIDIC_URL = "http://ftp.edrdg.org/pub/Nihongo/kanjidic2.xml.gz"

PRIMARY = {"news1", "ichi1", "spec1", "gai1"}


def kata_to_hira(s: str) -> str:
    return "".join(chr(ord(c) - 0x60) if 0x30A1 <= ord(c) <= 0x30F6 else c for c in s)


def hira_to_kata(s: str) -> str:
    return "".join(chr(ord(c) + 0x60) if 0x3041 <= ord(c) <= 0x3096 else c for c in s)


def nfc(s: str) -> str:
    return unicodedata.normalize("NFC", s)


def fetch_gz(url: str) -> bytes:
    print(f"fetching {url} ...", file=sys.stderr)
    with urllib.request.urlopen(url, timeout=180) as r:
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


def load_jmdict():
    """-> (reading(NFC hira) -> {keb: set(ke_pri)},  katakana_words: set(reading))."""
    xml = fetch_gz(JMDICT_URL)
    kanji_idx: dict = {}
    katakana_words: set = set()
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "entry":
            kebs = []
            for k in el.findall("k_ele"):
                keb = k.findtext("keb")
                if keb:
                    pris = {p.text for p in k.findall("ke_pri") if p.text}
                    kebs.append((keb, pris))
            rebs = [r.findtext("reb") for r in el.findall("r_ele")]
            rebs = [x for x in rebs if x]
            # A word is "katakana" only when ALL of an entry's readings are katakana —
            # i.e. an inherently katakana word (loanword/foreign: アメリカ, タイミング).
            # Native words that merely list a katakana variant (うっかり/ウッカリ) keep a
            # hiragana reading, so they are excluded.
            entry_all_kata = bool(rebs) and all(
                all(0x30A1 <= ord(c) <= 0x30FF or c == "ー" for c in x) for x in rebs
            )
            for reb in rebs:
                rh = nfc(kata_to_hira(reb))
                for keb, pris in kebs:
                    kanji_idx.setdefault(rh, {}).setdefault(keb, set()).update(pris)
                if entry_all_kata:
                    katakana_words.add(rh)
            el.clear()
    return kanji_idx, katakana_words


def load_jmnedict() -> dict:
    """JMnedict (proper nouns): reading(NFC hira) -> set of kanji writings (keb)."""
    xml = fetch_gz(JMNEDICT_URL)
    idx: dict = {}
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "entry":
            kebs = [k.text for k in el.findall("k_ele/keb") if k.text]
            for r in el.findall("r_ele/reb"):
                if r.text:
                    rh = nfc(kata_to_hira(r.text))
                    idx.setdefault(rh, set()).update(kebs)
            el.clear()
    return idx


def is_cjk(c: str) -> bool:
    o = ord(c)
    return 0x4E00 <= o <= 0x9FFF or 0x3400 <= o <= 0x4DBF


def is_kana(c: str) -> bool:
    o = ord(c)
    return 0x3041 <= o <= 0x309F or 0x30A1 <= o <= 0x30FF or c == "ー"


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
    return has_kanji and nfc(keb) == keb


def score(tags: set):
    primary = 1 if (tags & PRIMARY) else 0
    nfs = [int(t[2:]) for t in tags if t.startswith("nf") and t[2:].isdigit()]
    return (primary, -(min(nfs) if nfs else 999))


def main() -> int:
    words = [w for w in BIP39.read_text(encoding="utf-8").split("\n") if w]
    assert len(words) == 2048, f"expected 2048 BIP-39 words, got {len(words)}"
    jouyou = load_jouyou()
    kanji_idx, katakana_words = load_jmdict()
    jmnedict = load_jmnedict()
    print(f"jouyou:{len(jouyou)} jmdict-readings:{len(kanji_idx)} "
          f"katakana-words:{len(katakana_words)} jmnedict-readings:{len(jmnedict)}",
          file=sys.stderr)

    result, reasons = [], []
    for w in words:
        key = nfc(w)
        cands = kanji_idx.get(key, {})
        elig = [(k, t) for k, t in cands.items() if eligible(k, jouyou)]
        tagged = [(k, t) for k, t in elig if t]
        best, kind = None, "kana:no-eligible"
        if len(elig) == 1:
            best, kind = elig[0][0], "kanji"
        elif tagged:
            tagged.sort(key=lambda kt: (score(kt[1]), -len(kt[0]), kt[0]), reverse=True)
            best = tagged[0][0]
            alts = [k for k, _ in tagged[1:] if k != best][:3]
            kind = "kanji" + (f"(alt:{'/'.join(alts)})" if alts else "")
        elif key in katakana_words:
            best, kind = nfc(hira_to_kata(w)), "katakana"
        elif elig:
            kind = "kana:ambiguous-untagged"
        else:
            names = sorted({k for k in jmnedict.get(key, set()) if eligible(k, jouyou)})
            if len(names) == 1:
                best, kind = names[0], "kanji-name"

        result.append(best if best is not None else w)
        reasons.append(kind)

    # Collision resolution: any non-fallback value used by >1 index -> revert to kana.
    counts = Counter(result)
    for i, v in enumerate(result):
        if counts[v] > 1 and v != words[i]:
            result[i] = words[i]
            reasons[i] = "kana:collision"

    assert len(result) == 2048
    assert len(set(result)) == 2048, "post-conversion list is not unique"
    assert all(nfc(result[i]) == result[i] for i in range(2048) if result[i] != words[i]), \
        "a chosen kanji/katakana form is not NFC-stable"

    OUT.write_text("\n".join(result) + "\n", encoding="utf-8")
    REPORT.write_text(
        "\n".join(f"{i}\t{words[i]}\t{reasons[i]}\t{result[i]}" for i in range(2048)) + "\n",
        encoding="utf-8")
    kanji = sum(1 for r in reasons if r.startswith("kanji"))
    kata = sum(1 for r in reasons if r == "katakana")
    print(f"coverage: {kanji} kanji + {kata} katakana = {kanji + kata}/2048 "
          f"non-hiragana ({100 * (kanji + kata) / 2048:.1f}%)", file=sys.stderr)
    print(f"wrote {OUT} and {REPORT}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
