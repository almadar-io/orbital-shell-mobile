import { registerRootComponent } from 'expo';
import { Asset } from 'expo-asset';
import App from './src/App';

Asset.loadAsync([]);

registerRootComponent(App);
