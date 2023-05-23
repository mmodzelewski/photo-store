import { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
  appId: 'dev.modzelewski.photosstore',
  appName: 'Photos store',
  webDir: 'dist',
  server: {
    androidScheme: 'https'
  }
};

export default config;
