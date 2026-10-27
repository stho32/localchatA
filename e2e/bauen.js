'use strict';
// Vor jedem Lauf bauen, damit nie gegen eine veraltete eingebettete Oberfläche getestet wird.
const { execFileSync } = require('child_process');
const path = require('path');

module.exports = function bauen() {
  execFileSync('cargo', ['build', '-q'], { cwd: path.resolve(__dirname, '..'), stdio: 'inherit' });
};
