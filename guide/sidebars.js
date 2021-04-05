module.exports = {
  someSidebar: {
    About: [
        'about/guide',
        'about/dnp3',
        'about/features',
        'about/interop',
        'about/versioning',
        'about/license',
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
