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
        'languages/c_bindings',
        'languages/c_sharp',
    ],
    API: [
        'api/logging',
        'api/runtime',
        {
            Outstation: [
                'api/outstation/configuration',
                'api/outstation/controls',
                'api/outstation/application',
                'api/outstation/outstation_info',
                'api/outstation/database',
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
