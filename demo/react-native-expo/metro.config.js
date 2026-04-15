// Metro config for Expo + local symlinked packages (npm "file:" deps).
// This allows Metro to follow the symlink to ../../platforms/react-native.
const path = require('path');
const {getDefaultConfig} = require('expo/metro-config');

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, '../..');

const config = getDefaultConfig(projectRoot);

// Allow importing from outside the project root (monorepo / file: deps).
config.watchFolders = [
  path.resolve(workspaceRoot, 'platforms/react-native'),
  path.resolve(workspaceRoot, 'fonts'),
];

// Metro sometimes refuses to traverse symlinks unless explicitly enabled.
config.resolver.unstable_enableSymlinks = true;

// Prevent Metro from resolving dependencies (especially react/react-native)
// from the linked package's own node_modules, which can cause duplicate
// React Native versions and missing TurboModules at runtime.
const linkedNodeModules = path.resolve(
  workspaceRoot,
  'platforms/react-native/node_modules',
);
config.resolver.blockList = [
  new RegExp(`${linkedNodeModules.replace(/[/\\\\]/g, '[\\\\/]')}[/\\\\].*`),
];

const appNodeModules = path.resolve(projectRoot, 'node_modules');
config.resolver.extraNodeModules = new Proxy(
  {},
  {
    get: (_target, name) => path.join(appNodeModules, name),
  },
);

module.exports = config;

