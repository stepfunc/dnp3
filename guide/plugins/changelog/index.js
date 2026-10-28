// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

const fs = require('fs');
const path = require('path');

module.exports = function (context, options) {
    // Generate the page when the plugin is constructed: in loadContent it races the docs plugin's
    // scan, which misses the page on a clean checkout and fails the build.
    const filename = 'CHANGELOG.md';
    const source = fs.readFileSync(path.resolve(context.siteDir, `../${filename}`), 'utf8');
    // remark-emoji has no :book: shortcode and renders :shield: without the emoji variation
    // selector, so substitute the characters. The wrapper lets CSS style the entries.
    const body = source
        .replaceAll(':book:', '\u{1F4D6}')
        .replaceAll(':shield:', '\u{1F6E1}\u{FE0F}');
    fs.writeFileSync(
        path.resolve(context.siteDir, `docs/${filename}`),
        `<div className="changelog">\n\n${body}\n\n</div>\n`,
    );
    return {
        name: 'changelog',
    };
};
