# DCMfx - Core Library

Provides core DICOM concepts including data sets, data elements, value
representations, transfer syntaxes, and a registry of the data elements defined
in DICOM Part 6.

Part of the DCMfx project.

[![Package Version](https://img.shields.io/hexpm/v/dcmfx_core)](https://hex.pm/packages/dcmfx_core)
[![Hex Docs](https://img.shields.io/badge/hex-docs-ffaff3)](https://hexdocs.pm/dcmfx_core/)
![Erlang Compatible](https://img.shields.io/badge/target-erlang-a90432)
![JavaScript Compatible](https://img.shields.io/badge/target-javascript-f3e155)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Features

- Work with DICOM data element tags, values, and data sets.
- Parse data element values that have specific structures, including
  `AgeString`, `AttributeTag`, `Date`, `DateTime`, `DecimalString`,
  `IntegerString`, `PersonName`, `Time`, and `UniqueIdentifier` value
  representations.
- Look up the DICOM data element registry by tag, including well-known privately
  defined data elements.
- Retrieve pixel data from a data set with support for both basic and extended
  offset tables.
- Anonymize data sets by removing all data elements containing PHI.

API documentation is available [https://hexdocs.pm/dcmfx_core](here).

## License

DCMfx is published under the GNU Affero General Public License Version 3
(AGPLv3). This license permits commercial use; however, any software that
incorporates DCMfx, either directly or indirectly, must also be released under
the AGPLv3 or a compatible license. This includes making the source code of the
combined work available under the same terms, and ensuring that users who
interact with the software over a network can access the source code.

Copyright © Dr Richard Viney, 2024.
