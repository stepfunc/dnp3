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
                {
                    Components: [
                        'api/outstation/configuration',
                        'api/outstation/controls',
                        'api/outstation/application',
                        'api/outstation/outstation_info',
                    ],
                    Creation: [
                        'api/outstation/tcp_server',
                        'api/outstation/serial_outstation',
                    ],
                    Execution: [
                        'api/outstation/outstation',
                        'api/outstation/database',
                    ]
                }
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
