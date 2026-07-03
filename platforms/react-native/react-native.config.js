module.exports = {
  dependency: {
    platforms: {
      ios: {},
      macos: {},
      android: {
        sourceDir: './android',
        packageImportPath: 'import io.ratex.RaTeXPackage;',
        packageInstance: 'new RaTeXPackage()',
        cmakeListsPath: './src/main/jni/CMakeLists.txt',
      },
    },
  },
};
