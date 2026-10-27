// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

module.exports = {
  someSidebar: {
    About: [
        'about/guide',
        'about/dnp3',
        'about/interop',
        'about/security',
        'about/versioning',
        'about/license',
        'about/dependencies',
    ],
    Languages: [
        'languages/bindings',
        {
            Bindings: [
                'languages/c_bindings',
                'languages/cpp_bindings',
                'languages/java',
                'languages/c_sharp',
            ]
        }
    ],
    API: [
        'api/logging',
        'api/runtime',
        'api/tls',
        {
            Outstation: [
                'api/outstation/overview',
                'api/outstation/configuration',
                {
                    Interfaces: [
                        'api/outstation/controls',
                        'api/outstation/application',
                        'api/outstation/outstation_info',
                    ],
                    Transports: [
                        'api/outstation/tcp_server',
                        'api/outstation/tls_server',
                        'api/outstation/serial_outstation',
                    ],
                    Running: [
                        'api/outstation/outstation',
                        'api/outstation/database',
                    ]
                }
            ]
        },
        {
            Master: [
                'api/master/terminology',
                {
                    Channels: [
                        'api/master/channel_config',
                        'api/master/tcp_client',
                        'api/master/tls_client',
                        'api/master/serial_master',
                    ]
                },
                {
                    Associations: [
                        'api/master/assoc_create',
                        {
                            Components: [
                                'api/master/assoc_config',
                                'api/master/read_handler',
                                'api/master/association_handler',
                                'api/master/association_information',
                            ]
                        },
                        'api/master/assoc_polls',
                        'api/master/assoc_controls',
                        'api/master/attributes',
                        'api/master/file_transfer',
                        'api/master/assoc_other',
                    ]
                },
                'api/master/scheduling',
            ]
        }
    ],
    Examples: [
        'examples/summary'
    ],
  },
};
