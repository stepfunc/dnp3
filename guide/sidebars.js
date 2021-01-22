module.exports = {
  someSidebar: {
    About: [
        'about/guide',
        'about/dnp3',
        'about/library',
        'about/interop',
        'about/versioning',
        'about/license',
    ],
    API: [
        'api/logging',
        'api/runtime',
    ],
    Examples: [
        'examples/summary',
        {
            Rust: ['examples/rust/tcp_master', 'examples/rust/tcp_outstation'],
        }
    ],
  },
};
