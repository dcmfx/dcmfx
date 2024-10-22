::: tip UNRELEASED
This library is not yet published as a package or crate.
:::

::: warning
:construction: Under construction :construction:
:::

# `dcmfx_core`

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
