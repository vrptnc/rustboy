import React from 'react';
import ReactDOM from 'react-dom';
import { App } from './components/app/app';
import { default as init} from '../pkg/rustboy'
import rustboyWasm from '../pkg/rustboy_bg.wasm'

declare global {
  interface Window {
    webkitAudioContext: AudioContext
  }
}

init(rustboyWasm).then(() => {
  ReactDOM.render(<App />, document.getElementById('root'));
})
