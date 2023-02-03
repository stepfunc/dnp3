module.exports = {
  someSidebar: {
    About: [
        'about/guide',
        'about/dnp3',
        'about/interop',
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
            ]
        }
    ],
    Examples: [
        'examples/summary'
    ],
  },
};
