const fs = require('fs');
const path = require('path');
const visit = require('unist-util-visit');

function findSample(filename, anchor) {
    const file = fs.readFileSync(filename, 'utf8');
    const start_regex = /ANCHOR:\s*([\w_-]+)/;
    const end_regex = /ANCHOR_END:\s*([\w_-]+)/;

    if(anchor) {
        let found = false;
        let lines = [];
        let min_num_whitespaces = null;
        for (const line of file.split(/\r?\n/)) {
            if(!found) {
                const match = line.match(start_regex)
                if(match && match[1] === anchor) {
                    found = true;
                }
            }
            else {
                // Check if we found end anchor
                const match = line.match(end_regex)
                if(match && match[1] === anchor) {
                    break;
                }
                else {
                    // Check whitespaces
                    const num_whitespaces = line.match(/([ ]*).*/)[1].length;
                    if(!min_num_whitespaces || num_whitespaces < min_num_whitespaces) {
                        min_num_whitespaces = num_whitespaces;
                    }

                    // Push the line
                    lines.push(line);
                }
            }
        };

        if(!found) {
            throw new Error(`Could not find '${anchor}' anchor in ${filename}.`);
        }

        let result = '';
        lines.flatMap((line, index) => {
            result += line.substring(min_num_whitespaces);
            if(index + 1 != lines.length) {
                result += "\n";
            }
        });
        return result;
    } else {
        return file;
    }
}

const RE_PARENS = new RegExp(''
    + '\{\{\\s*'                   // link opening parens and whitespace
    + '\#([a-zA-Z0-9_]+)'          // link type
    + '\\s+'                       // separating whitespace
    + '([a-zA-Z0-9\s_.\\-:/\\\+]+)' // link target path and space separated properties
    + '\\s*\}\}'                   // whitespace and link closing parens"
, 'g');

module.exports = function codeSample(options = {}) {
    return (tree, file) => {
        const codes = [];

        visit(tree, 'code', (node, index, parent) => {
            codes.push([node, index, parent]);
        });

        for (const [node] of codes) {
            const matches = node.value.matchAll(RE_PARENS);

            let result = '';
            let current_idx = 0;
            for (const match of matches) {
                // Check if it's an include
                if (match[1] == 'include') {
                    const [filepath, anchor] = match[2].split(':');

                    // Copy everything before the tag
                    result += node.value.substr(current_idx, match.index - current_idx);

                    // Copy the modified text
                    result += findSample(path.resolve(__dirname, '../../', filepath), anchor);

                    // Update the current index
                    current_idx = match.index + match[0].length;
                }
            }

            result += node.value.substr(current_idx, node.value.length - current_idx);

            node.value = result;
        }
  };
}
