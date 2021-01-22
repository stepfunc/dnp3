const remarkPlugin = require('./plugins/remark');

module.exports = {
  title: 'DNP3 <version>',
  tagline: 'Pretty sure we don\'t need this page, just the docs',
  url: 'https://your-docusaurus-test-site.com',
  baseUrl: '/',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/favicon.ico',
  organizationName: 'stepfunc', // Usually your GitHub org/user name.
  projectName: 'dnp3', // Usually your repo name.
  themeConfig: {
    prism: {
      additionalLanguages: ['rust', 'java', 'csharp'],
    },
    navbar: {
      title: 'DNP3 <version>',
      logo: {
        alt: 'Logo',
        src: 'img/logo.svg',
      },
      items: [
        {
          to: 'docs/dnp3',
          activeBasePath: 'docs',
          label: 'Docs',
          position: 'left',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
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
        {
          title: 'Library',
          items: [
            {
              label: 'Github',
              href: 'https://stackoverflow.com/questions/tagged/docusaurus',
            },
            {
              label: 'Homepage',
              href: 'https://stepfunc.io/products/libraries/dnp3/',
            },
          ],
        },
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
            remarkPlugin,
          ],
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
};
