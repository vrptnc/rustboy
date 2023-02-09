import React from 'react';
import ReactDOM from 'react-dom';
import { App } from './components/app/app';

declare global {
  interface Window {
    webkitAudioContext: AudioContext
  }
}

ReactDOM.render(<App />, document.getElementById('root'));