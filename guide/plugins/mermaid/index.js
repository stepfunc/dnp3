const path = require('path');
const visit = require('unist-util-visit');

module.exports = function mermaid(options = {}) {
    return (tree, file) => {
        let importAdded = false;
        visit(tree, 'code', (node, index, parent) => {
            if(node.lang === 'mermaid') {
                node.type = 'jsx';
                node.value = `<Mermaid chart="${node.value}" />`;

                if(!importAdded) {
                    const importPath = path.relative(file.dirname, path.resolve(__dirname, '../../src/theme/Mermaid')).replace(/\\/g, '/');
                    const importNode = {
                        type: 'import',
                        value: `import Mermaid from '${importPath}'`,
                    }
                    parent.children.splice(index, 0, importNode);
                    importAdded = true;

                    return index + 1
                }
            }
        });
  };
}
