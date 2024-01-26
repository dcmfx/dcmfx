# DCMfx - Character Set Library

Decodes DICOM string data that uses a Specific Character Set into a native UTF-8
string.

Part of the DCMfx project.

[![Package Version](https://img.shields.io/hexpm/v/dcmfx_character_set)](https://hex.pm/packages/dcmfx_character_set)
[![Hex Docs](https://img.shields.io/badge/hex-docs-ffaff3)](https://hexdocs.pm/dcmfx_character_set/)
![Erlang Compatible](https://img.shields.io/badge/target-erlang-a90432)
![JavaScript Compatible](https://img.shields.io/badge/target-javascript-f3e155)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Features

This library exposes definitions for all DICOM character sets and supports
decoding of all Specific Character Sets defined by the DICOM standard, including
those that use Code Extensions via ISO 2022 escape sequences.

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

Invalid bytes are replaced with the � (U+FFFD) character in the returned string.

API documentation is available [https://hexdocs.pm/dcmfx_character_set](here).

## License

DCMfx is published under the GNU Affero General Public License Version 3
(AGPLv3). This license permits commercial use; however, any software that
incorporates DCMfx, either directly or indirectly, must also be released under
the AGPLv3 or a compatible license. This includes making the source code of the
combined work available under the same terms, and ensuring that users who
interact with the software over a network can access the source code.

Copyright © Dr Richard Viney, 2024.
