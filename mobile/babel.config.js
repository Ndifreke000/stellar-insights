module.exports = {
  presets: ['module:@react-native/babel-preset'],
  plugins: [
    [
      'module-resolver',
      {
        root: ['./src'],
        extensions: ['.ios.js', '.android.js', '.js', '.ts', '.tsx', '.json'],
        alias: {
          '@': './src',
          '@components': './src/components',
          '@screens': './src/screens',
          '@services': './src/services',
          '@hooks': './src/hooks',
          '@utils': './src/utils',
          '@features': './src/features',
          '@app-types': './src/types',
          '@store': './src/store',
          '@config': './src/config',
          '@navigation': './src/navigation',
        },
      },
    ],
    // Reanimated 4 moved its Babel plugin into react-native-worklets
    // (react-native-reanimated/plugin now just requires it). It must be last.
    'react-native-worklets/plugin',
  ],
};
