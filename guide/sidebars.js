/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  someSidebar: [
    {
      type: 'category',
      label: 'About',
      items: [
        'about/guide',
        'about/dnp3',
        'about/interop',
        'about/versioning',
        'about/license',
        'about/dependencies',
      ],
    },
    {
      type: 'category',
      label: 'Languages',
      items: [
        'languages/bindings',
        {
          type: 'category',
          label: 'Bindings',
          items: [
            'languages/c_bindings',
            'languages/cpp_bindings',
            'languages/java',
            'languages/c_sharp',
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'API',
      items: [
        'api/logging',
        'api/runtime',
        'api/tls',
        {
          type: 'category',
          label: 'Outstation',
          items: [
            'api/outstation/configuration',
            {
              type: 'category',
              label: 'Interfaces',
              items: [
                'api/outstation/controls',
                'api/outstation/application',
                'api/outstation/outstation_info',
              ],
            },
            {
              type: 'category',
              label: 'Transports',
              items: [
                'api/outstation/tcp_server',
                'api/outstation/tls_server',
                'api/outstation/serial_outstation',
              ],
            },
            {
              type: 'category',
              label: 'Running',
              items: [
                'api/outstation/outstation',
                'api/outstation/database',
              ],
            },
          ],
        },
        {
          type: 'category',
          label: 'Master',
          items: [
            'api/master/terminology',
            {
              type: 'category',
              label: 'Channels',
              items: [
                'api/master/channel_config',
                'api/master/tcp_client',
                'api/master/tls_client',
                'api/master/serial_master',
              ],
            },
            {
              type: 'category',
              label: 'Associations',
              items: [
                'api/master/assoc_create',
                {
                  type: 'category',
                  label: 'Components',
                  items: [
                    'api/master/assoc_config',
                    'api/master/read_handler',
                    'api/master/association_handler',
                    'api/master/association_information',
                  ],
                },
                'api/master/assoc_polls',
                'api/master/assoc_controls',
                'api/master/attributes',
                'api/master/file_transfer',
                'api/master/assoc_other',
              ],
            },
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'Examples',
      items: [
        'examples/summary',
      ],
    },
  ],
};

module.exports = sidebars;