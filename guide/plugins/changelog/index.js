const fs = require('fs/promises');
const path = require('path');

module.exports = function (context, options) {
    return {
        name: 'changelog',
        async loadContent() {
            const filename = 'CHANGELOG.md';
            await fs.copyFile(path.resolve(context.siteDir, `../${filename}`), path.resolve(context.siteDir, `docs/${filename}`));
        },
    };
};
