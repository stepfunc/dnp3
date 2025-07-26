const path = require('path');
const samplePlugin = require('./plugins/sample');
const sitedata = require('./sitedata.json');
const {themes} = require('prism-react-renderer');
const vsLight = themes.vsLight;

module.exports = {
  title: `DNP3 ${sitedata.version}`,
  tagline: 'Pretty sure we don\'t need this page, just the docs',
  url: 'https://docs.stepfunc.io',
  baseUrl: `/dnp3/${sitedata.version}/guide/`,
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'images/brand/favicon.png',
  organizationName: 'stepfunc', // Usually your GitHub org/user name.
  projectName: 'dnp3', // Usually your repo name.
  themeConfig: {
    prism: {
      theme: vsLight,
      additionalLanguages: ['rust', 'java', 'csharp', 'cmake'],
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: true,
    },
    navbar: {
      title: `DNP3 ${sitedata.version}`,
      logo: {
        alt: 'Logo',
        src: 'images/brand/logo.svg',
        href: '/docs/guide'
      },
      items: [],
    },
    footer: {
      logo: {
        alt: 'Step Function',
        src: 'images/brand/footer-logo.svg',
      },
      links: [
        {
          title: 'Step Function I/O',
          items: [
            {
              label: 'Products',
              href: 'https://stepfunc.io/products/',
            },
            {
              label: 'Blog',
              to: 'https://stepfunc.io/blog/',
            },
          ],
        },
        {
          title: 'Library',
          items: [
            {
              label: 'GitHub',
              href: sitedata.github_url,
            },
            {
              label: 'Homepage',
              href: 'https://stepfunc.io/products/libraries/dnp3/',
            },
          ],
        },
        {
          title: 'DNP3',
          items: [
            {
              label: 'User Group',
              to: 'https://dnp.org',
            },
            {
              label: 'IEEE 1815-2012',
              to: 'https://standards.ieee.org/standard/1815-2012.html',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Step Function I/O LLC`,
    },
    mermaid: {
      theme: {
        light: 'base',
        dark: 'dark',
      },
      options: {
        themeVariables: {
          // Primary colors matching the guide
          primaryColor: '#e4f3ff',
          primaryTextColor: '#004e98',
          primaryBorderColor: '#004e98',
          
          // Line and edge colors
          lineColor: '#0056a7',
          
          // Background colors
          background: '#ffffff',
          mainBkg: '#e4f3ff',
          secondBkg: '#f5f6f7',
          
          // Font
          fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, Ubuntu, Cantarell, Noto Sans, sans-serif',
          fontSize: '14px',
          
          // Flowchart specific
          nodeBkg: '#e4f3ff',
          nodeTextColor: '#004e98',
          edgeLabelBackground: '#ffffff',
          
          // Sequence diagram specific
          actorBkg: '#e4f3ff',
          actorBorder: '#004e98',
          actorTextColor: '#004e98',
          signalColor: '#004e98',
          signalTextColor: '#004e98',
        },
      },
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          remarkPlugins: [
            samplePlugin,
          ],
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
  themes: ['@docusaurus/theme-mermaid'],
  markdown: {
    mermaid: true,
  },
  plugins: [path.resolve(__dirname, './plugins/changelog')],
};
