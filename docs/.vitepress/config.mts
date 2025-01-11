import { defineConfig } from "vitepress";
import { tabsMarkdownPlugin } from "vitepress-plugin-tabs";

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
        '<small>Copyright © 2024-2025, <a href="mailto:richard.viney@gmail.com">Dr Richard Viney</a></small>',
    },

    search: {
      provider: "local",
    },

    nav: [
      { text: "Home", link: "/" },
      { text: "Tools", link: "/tools/overview" },
      { text: "Libraries", link: "/libraries/overview" },
    ],

    sidebar: [
      { text: "Introduction", link: "/introduction" },
      { text: "Design", link: "/design" },
      { text: "Conformance", link: "/conformance" },
      { text: "Roadmap", link: "/roadmap" },
      { text: "License", link: "/license" },
      {
        text: "DICOM Tools",
        items: [
          { text: "Overview", link: "/tools/overview" },
          { text: "CLI Tool", link: "/tools/cli" },
          { text: "VS Code Extension", link: "/tools/vs-code-extension" },
          { text: "Playground", link: "/tools/playground" },
        ],
      },
      {
        text: "DICOM Libraries",
        items: [
          { text: "Overview", link: "/libraries/overview" },
          {
            text: "Examples",
            link: "/libraries/examples",
            collapsed: true,
            items: [
              {
                text: "Read DICOM",
                link: "/libraries/examples/dicom-read",
              },
              {
                text: "Write DICOM",
                link: "/libraries/examples/dicom-write",
              },
              {
                text: "Stream DICOM",
                link: "/libraries/examples/dicom-stream",
              },
              {
                text: "Read DICOM JSON",
                link: "/libraries/examples/dicom-json-read",
              },
              {
                text: "Write DICOM JSON",
                link: "/libraries/examples/dicom-json-write",
              },
              {
                text: "Anonymize Data Set",
                link: "/libraries/examples/data-set-anonymize",
              },
              {
                text: "Read Pixel Data",
                link: "/libraries/examples/pixel-data-read",
              },
            ],
          },
        ],
      },
      { text: "Acknowledgements", link: "/acknowledgements" },
    ],

    socialLinks: [{ icon: "github", link: "https://github.com/dcmfx" }],
  },

  markdown: {
    config(md) {
      md.use(tabsMarkdownPlugin);
    },
  },

  sitemap: {
    hostname: "https://dcmfx.github.io",
  },
});
