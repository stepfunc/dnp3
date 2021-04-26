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
                'languages/java',
                'languages/c_sharp',
            ]
        }
    ],
    API: [
        'api/logging',
        'api/runtime',
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
                    Configuration: [
                        'api/master/channel_config',
                        'api/master/assoc_config',
                    ]
                },
                {
                    Interfaces: [
                        'api/master/read_handler',
                        'api/master/association_handler',
                    ]
                },
                {
                    Channels: [
                        'api/master/tcp_client',
                        'api/master/serial_master',
                    ]
                },
                {
                    Associations: [
                        'api/master/assoc_create',
                    ]
                },
            ]
        }
    ],
    Examples: [
        'examples/summary',
        {
            Rust: ['examples/rust/tcp_master', 'examples/rust/tcp_outstation'],
        }
    ],
  },
};
