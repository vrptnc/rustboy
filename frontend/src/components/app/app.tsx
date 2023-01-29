import {Emulator} from '../../../rustboy';
import React, {FormEvent, useEffect, useRef, useState} from 'react'
import './app.scss'
// @ts-ignore
import gbImage from '../../images/gb.png'

export const App = () => {

  const [emulator, setEmulator] = useState<Emulator>()
  const previousTimeRef = useRef<number>()
  const animationFrameId = useRef<number>()

  const execute = () => {
    let currentTime = performance.now();
    let delta_ms = previousTimeRef.current ? (currentTime - previousTimeRef.current) : 1
    emulator?.tick(BigInt(Math.floor(delta_ms * 1_000_000)))
    previousTimeRef.current = currentTime
    animationFrameId.current = requestAnimationFrame(execute)
  }

  useEffect(() => {
    if (animationFrameId.current != null) {
      cancelAnimationFrame(animationFrameId.current)
    }
    if (emulator != null) {
      requestAnimationFrame(execute)
    }
  }, [emulator])

  const handleRomChange = async (event: FormEvent<HTMLInputElement>) => {
    const files = event.currentTarget.files;
    if (files != null && files.length > 0) {
      const file = files.item(0)
      if (file != null) {
        const arrayBuffer = await file.arrayBuffer()
        const byteArray = new Uint8Array(arrayBuffer);
        setEmulator(Emulator.new(byteArray))
      }
    }
  }

  return <div className="app">
    <h1>RustBoy</h1>
    <div className="button-wrapper">
      <label className="button" htmlFor="rom_selector">Choose ROM</label>
      <input
        className="hidden"
        type="file"
        id="rom_selector"
        name="rom_selector"
        accept=".gb, .gbc"
        onChange={ handleRomChange }/>
    </div>
    <div className="visualizer">
      <img width={ "361px" } height={ "621px" } src={ gbImage }></img>
      <canvas id="main-canvas" width={ 160 } height={ 144 }></canvas>
    </div>


  </div>
}