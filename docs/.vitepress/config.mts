import { defineConfig } from "vitepress";

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "DCMfx",
  description: "Tools and libraries for working with DICOM",

  srcDir: "src",

  cleanUrls: true,

  head: [["link", { rel: "icon", href: "/favicon.ico" }]],

  // https://vitepress.dev/reference/default-theme-config
  themeConfig: {
    logo: "/logo.png",

    footer: {
      copyright:
        'Copyright © 2024 <a href="https://github.com/richard-viney">Dr Richard Viney</a>',
    },

    search: {
      provider: "local",
    },

    nav: [
      { text: "Home", link: "/" },
      { text: "Tools", link: "/cli-tool" },
    ],

    sidebar: [
      {
        text: "Introduction",
        items: [
          { text: "About", link: "/about" },
          { text: "License", link: "/license" },
        ],
      },
      {
        text: "Tools",
        items: [
          { text: "CLI Tool", link: "/cli-tool" },
          { text: "VS Code Extension", link: "/vs-code-extension" },
          { text: "Playground", link: "/playground" },
        ],
      },
      {
        text: "Libraries",
        items: [
          {
            text: "Libraries",
            link: "/libraries",
            collapsed: true,
            items: [
              { text: "dcmfx_core", link: "libraries/dcmfx-core" },
              { text: "dcmfx_p10", link: "libraries/dcmfx-p10" },
              { text: "dcmfx_json", link: "libraries/dcmfx-json" },
              { text: "dcmfx_pixel_data", link: "libraries/dcmfx-pixel-data" },
              { text: "dcmfx_anonymize", link: "libraries/dcmfx-anonymize" },
              { text: "dcmfx_character_set", link: "libraries/dcmfx-character-set" },
            ],
          },
          { text: "Languages", link: "/languages" },
        ],
      },
      {
        text: "Other",
        items: [{ text: "Acknowledgements", link: "/acknowledgements" }],
      },
    ],

    socialLinks: [{ icon: "github", link: "https://github.com/dcmfx" }],
  },
});
