::: tip UNRELEASED
This library is not yet published as a package or crate.
:::

# `dcmfx_character_set`

This internal library exposes definitions for all DICOM character sets and
supports decoding of all Specific Character Sets defined by the DICOM standard,
including those that use Code Extensions via ISO 2022 escape sequences.

Invalid bytes are replaced with the � (U+FFFD) character in the returned string.

The list of supported encodings is:

- ISO_IR 6 (ISO 646, US-ASCII)
- ISO_IR 100 (ISO 8859-1, Latin-1)
- ISO_IR 101 (ISO 8859-2, Latin-2)
- ISO_IR 109 (ISO 8859-3, Latin-3)
- ISO_IR 110 (ISO 8859-4, Latin-4)
- ISO_IR 144 (ISO 8859-5, Latin/Cyrillic)
- ISO_IR 127 (ISO 8859-6, Latin/Arabic)
- ISO_IR 126 (ISO 8859-7, Latin/Greek)
- ISO_IR 138 (ISO 8859-8, Latin/Hebrew)
- ISO_IR 148 (ISO 8859-9, Latin-5)
- ISO_IR 203 (ISO 8859-15, Latin-9)
- ISO_IR 13 (JIS X 0201)
- ISO_IR 166 (ISO 8859-11, TIS 620-2533)
- ISO 2022 IR 6
- ISO 2022 IR 100 (ISO 8859-1, Latin-1)
- ISO 2022 IR 101 (ISO 8859-2, Latin-2)
- ISO 2022 IR 109 (ISO 8859-3, Latin-3)
- ISO 2022 IR 110 (ISO 8859-4, Latin-4)
- ISO 2022 IR 144 (ISO 8859-5, Latin/Cyrillic)
- ISO 2022 IR 127 (ISO 8859-6, Latin/Arabic)
- ISO 2022 IR 126 (ISO 8859-7, Latin/Greek)
- ISO 2022 IR 138 (ISO 8859-8, Latin/Hebrew)
- ISO 2022 IR 148 (ISO 8859-9, Latin-5)
- ISO 2022 IR 203 (ISO 8859-15, Latin-9)
- ISO 2022 IR 13 (JIS X 0201)
- ISO 2022 IR 166 (ISO 8859-11, TIS 620-2533)
- ISO 2022 IR 87 (JIS X 0208)
- ISO 2022 IR 159 (JIS X 0212)
- ISO 2022 IR 149 (KS X 1001)
- ISO 2022 IR 58 (GB 2312)
- ISO_IR 192 (UTF-8)
- GB18030
- GBK
