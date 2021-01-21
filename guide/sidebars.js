module.exports = {
  someSidebar: {
    About: [
        'about/guide',
        'about/dnp3',
        'about/library',
        'about/license',
    ],
    API: ['api/logging'],
    Examples: [
        'examples/summary',
        {
            Rust: ['examples/rust/tcp_master', 'examples/rust/tcp_outstation'],
        }
    ],
  },
};
