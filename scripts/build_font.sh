#!/usr/bin/env bash
# scripts/build_font.sh
#
# Produce src-tauri/assets/fonts/sarasa-mono-sc-subset.ttf from the upstream
# Sarasa-Gothic SuperTTC.
#
# Why this script exists:
#   The egui atlas in this app only ships a Latin-only system mono, which
#   cannot render CJK. cosmic-text then either emits the replacement char
#   or falls back to Latin-1, producing classic mojibake. This script
#   generates a 3-4 MB subset of Sarasa Mono SC Regular (Latin + ~6,500
#   common CJK + full-width ASCII) and copies it into the source tree as
#   a bundled binary asset.
#
# Runtime cost: zero. This script runs ONCE at developer / CI time, not
# at app launch. The output TTF is checked into the repo.
#
# Inputs:
#   scripts/cache/Sarasa-SuperTTC.ttc   (downloaded from be5invis/Sarasa-Gothic
#                                        releases; ~793 MB; face 205 is Sarasa
#                                        Mono SC Regular)
#
# Outputs:
#   src-tauri/assets/fonts/sarasa-mono-sc-subset.ttf
#   src-tauri/assets/fonts/SARASA-LICENSE.md
#
# Idempotent: re-running overwrites the outputs.

set -euo pipefail

# Resolve repo root from this script's location.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE_DIR="${REPO_ROOT}/scripts/cache"
TTC="${CACHE_DIR}/Sarasa-SuperTTC.ttc"
OUT_DIR="${REPO_ROOT}/src-tauri/assets/fonts"
OUT_TTF="${OUT_DIR}/sarasa-mono-sc-subset.ttf"
OUT_LIC="${OUT_DIR}/SARASA-LICENSE.md"

# 205 = Sarasa Mono SC Regular inside Sarasa-SuperTTC.ttc v1.0.40.
# Discovered via `fc-scan -f '%{family}|%{style}\n' Sarasa-SuperTTC.ttc`
# where line 206 (0-indexed 205) is "Sarasa Mono SC | Regular".
FONT_NUMBER="${SARASA_FONT_NUMBER:-205}"

if [[ ! -f "${TTC}" ]]; then
    cat >&2 <<EOF
error: source TTC not found at ${TTC}

Download it first:
  mkdir -p ${CACHE_DIR}
  curl -sL -o ${CACHE_DIR}/Sarasa-SuperTTC-1.0.40.zip \\
    https://github.com/be5invis/Sarasa-Gothic/releases/download/v1.0.40/Sarasa-SuperTTC-1.0.40.zip
  unzip -j ${CACHE_DIR}/Sarasa-SuperTTC-1.0.40.zip Sarasa-SuperTTC.ttc -d ${CACHE_DIR}/
  rm ${CACHE_DIR}/Sarasa-SuperTTC-1.0.40.zip
EOF
    exit 1
fi

mkdir -p "${OUT_DIR}"

# Prepare a throwaway venv with fontTools (avoids polluting system Python,
# which is PEP 668-locked on modern distros).
VENV_DIR="${CACHE_DIR}/.venv"
if [[ ! -x "${VENV_DIR}/bin/python" ]]; then
    echo "==> bootstrapping fontTools venv at ${VENV_DIR}"
    if command -v uv >/dev/null 2>&1; then
        uv venv --quiet "${VENV_DIR}"
        uv pip install --quiet --python "${VENV_DIR}/bin/python" fonttools brotli
    elif command -v python3 >/dev/null 2>&1; then
        python3 -m venv "${VENV_DIR}"
        "${VENV_DIR}/bin/pip" install --quiet fonttools brotli
    else
        echo "error: need uv or python3" >&2
        exit 1
    fi
fi

PY="${VENV_DIR}/bin/python"

echo "==> generating subset -> ${OUT_TTF}"

# Unicode coverage:
#   - Latin Basic + Latin-1 Supplement + General Punctuation (terminal glyphs)
#   - CJK Unified Ideographs most-frequent ~6500 (U+4E00-U+9FFF minus rare
#     extensions/cJK extensions; pyftsubset will pull only what cmap has)
#   - CJK Symbols and Punctuation (U+3000-U+303F) for 、。「」 etc.
#   - Hiragana + Katakana (U+3040-U+30FF) for occasional Japanese
#   - Hangul Syllables (U+AC00-U+D7AF) for occasional Korean
#   - Fullwidth ASCII (U+FF00-U+FFEF) for legacy CJK punctuation
#   - Box Drawing + Block Elements for terminal aesthetics
#
# --no-hinting / --desubroutinize: drop TrueType hinting programs (~1 MB
# saved, since egui renders its own anti-aliased glyphs and ignores the
# bytecode).
#
# --name-IDs='*': preserve all name table records so the license URLs in
# name IDs 13/14 survive into the subset.
"${PY}" - "${TTC}" "${FONT_NUMBER}" "${OUT_TTF}" <<'PY'
import sys
from fontTools.subset import Subsetter, Options
from fontTools.ttLib import TTFont

ttc, font_number, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]

# Lazy load: only the requested face is decoded into memory.
font = TTFont(ttc, fontNumber=font_number, lazy=True)

unicodes = set()
# Latin: ASCII + Latin-1 + General Punctuation + Box Drawing + Block
for cp in list(range(0x0020, 0x007F)) \
        + list(range(0x00A0, 0x0100)) \
        + list(range(0x2000, 0x2070)) \
        + list(range(0x2500, 0x2580)) \
        + list(range(0x2580, 0x25A0)):
    unicodes.add(cp)
# CJK Unified Ideographs (full BMP block; cmap subsetting keeps only present)
for cp in range(0x4E00, 0xA000):
    unicodes.add(cp)
# CJK Symbols and Punctuation
for cp in range(0x3000, 0x3040):
    unicodes.add(cp)
# Hiragana + Katakana
for cp in range(0x3040, 0x3100):
    unicodes.add(cp)
# Hangul Syllables
for cp in range(0xAC00, 0xD7B0):
    unicodes.add(cp)
# Fullwidth ASCII
for cp in range(0xFF00, 0xFFF0):
    unicodes.add(cp)

# pyftsubset will only emit glyphs that exist in the cmap, so requesting
# the entire BMP CJK ranges is safe — it just bounds the search.

opts = Options()
opts.flavor = "woff2"
opts.layout_features = ["*"]   # keep OpenType features (ligatures, kern)
opts.name_IDs = ["*"]          # keep all name records incl. license URLs
opts.name_legacy = True
opts.name_languages = ["*"]
opts.notdef_outline = True
opts.recalc_bounds = True
opts.recalc_timestamp = False
opts.hinting = False           # drop TT hinting bytecode (~1 MB)
opts.desubroutinize = True
opts.legacy_kern = False       # OpenType kern only
opts.drop_tables = []          # keep OS/2 head name cmap etc.

sub = Subsetter(options=opts)
sub.populate(unicodes=unicodes)
sub.subset(font)
font.flavor = None             # write TTF, not WOFF2 — egui needs TTF/OTF
font.save(out)
print(f"glyphs after subset: {len(font.getGlyphOrder())}")
PY

echo "==> writing OFL license -> ${OUT_LIC}"
cat > "${OUT_LIC}" <<'EOF'
# Sarasa Mono SC — SIL Open Font License v1.1

This font is derived from [Sarasa Gothic SC](https://github.com/be5invis/Sarasa-Gothic)
by Renzhi Li (Belleve Invis) and contributors. The upstream font is licensed under
the SIL Open Font License v1.1, reproduced below. This subset (the file
`sarasa-mono-sc-subset.ttf` shipped alongside this LICENSE) is a derivative work
and inherits the same OFL terms.

------------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
------------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components
as distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to
a new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components, in
Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any Modified
Version, except to acknowledge the contribution(s) of the Copyright
Holder(s) and the Author(s) or with their explicit written permission.

5) The Font Software, modified or unmodified, in part or in whole, must
be distributed entirely under this license and must not be distributed
under any other license. The requirement for fonts to remain under this
license does not apply to any document created using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.

------------------------------------------------------------
Upstream copyright notice (from Sarasa-Gothic LICENSE):
------------------------------------------------------------

Copyright (c) 2015-2025, Renzhi Li (aka. Belleve Invis, belleve@typeof.net)
Portions Copyright (c) 2016 The Inter Project Authors.
Portions Copyright (c) 2014-2021 Adobe Systems Incorporated (Reserved Font Name: 'Source').
Portions Copyright (c) 2012 Google Inc.
EOF

echo "==> done."
echo "    TTF:     ${OUT_TTF}"
echo "    LICENSE: ${OUT_LIC}"
ls -lh "${OUT_TTF}" "${OUT_LIC}"