import * as wasm from './rustboy_bg.wasm';
import React, {useEffect} from 'react'
import './app.scss'

export const App = () => {

  useEffect(() => {
    wasm.run_emulator()
  }, [])

  return <div>Hello world!</div>
}