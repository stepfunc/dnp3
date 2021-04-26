const path = require('path');
const samplePlugin = require('./plugins/sample');
const mermaidPlugin = require('./plugins/mermaid');
const sitedata = require('./sitedata.json');

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
      theme: require('prism-react-renderer/themes/vsLight'),
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
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          remarkPlugins: [
            samplePlugin,
            mermaidPlugin,
          ],
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
  plugins: [path.resolve(__dirname, './plugins/changelog')],
};
