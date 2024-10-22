---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "DCMfx"
  text:
  tagline: Tools and libraries for working with DICOM
  actions:
    - theme: brand
      text: Learn More
      link: /about
    - theme: alt
      text: Tools
      link: /cli-tool
    - theme: alt
      text: GitHub
      link: https://github.com/dcmfx

features:
  - title: CLI Tool
    icon: ⚡
    details: |
      A super-fast DICOM CLI tool with extremely low memory usage. Written in
      Rust and available for all platforms.

  - title: VS Code Extension
    icon: 🧩
    details: |
      DCMfx's VS Code extension lets you view and convert DICOM and DICOM JSON
      files directly in Visual Studio Code.

  - title: DICOM Streaming
    icon: 🌊
    details: |
      Stream decode and encode DICOM data. Read, modify, add, and remove data
      elements as they stream through.

  - title: Compliant
    icon: ✅
    details: |
      Supports all DICOM files, every transfer syntax, truncated and corrupted
      data, and reads all character sets.
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #3498db 30%, #61d1ff);;
}
</style>