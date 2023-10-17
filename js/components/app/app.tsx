import {Button, CPUInfo, WebEmulator} from '../../../pkg/rustboy';
import React, {Fragment, KeyboardEvent, useEffect, useRef, useState} from 'react'
import './app.scss'
// @ts-ignore
import gbImage from '../../images/gb.png'
import {GameBoy} from "../gameboy/gameboy";
import {ButtonBar} from "../button-bar/button-bar";
import {TabPane} from "../tab-pane/tab-pane";

export const App = () => {

  const [emulator, setEmulator] = useState<WebEmulator>()

  const [cpuInfo, setCPUInfo] = useState<CPUInfo>()
  const previousTimeRef = useRef<number>()
  const animationFrameId = useRef<number>()
  const buttonMapping: Record<string, Button> = {
    KeyW: Button.UP,
    KeyA: Button.LEFT,
    KeyS: Button.DOWN,
    KeyD: Button.RIGHT,
    KeyB: Button.A,
    KeyN: Button.B,
    KeyT: Button.START,
    KeyY: Button.SELECT
  }

  const scheduleRun = () => {
    animationFrameId.current = requestAnimationFrame(execute)
  }

  const execute = () => {
    const currentTime = performance.now()
    const previousTime = previousTimeRef.current ?? (currentTime - 1)
    const deltaMilliseconds = currentTime - previousTime
    const deltaNanoseconds = BigInt(Math.floor(deltaMilliseconds * 1_000_000))
    emulator?.run_for_nanos(deltaNanoseconds)
    previousTimeRef.current = currentTime
    scheduleRun()
  }

  // const doTick = () => {
  //   if (paused) {
  //     emulator?.execute_machine_cycle()
  //     const info = emulator?.cpu_info();
  //     setCPUInfo(info)
  //   }
  // }

  // const saveState = () => {
  //   if (emulator) {
  //     setPaused(true)
  //     const state = emulator.get_state()
  //     const blob = new Blob([state.buffer], {
  //       type: 'application/octet-stream'
  //     })
  //     saveAs(blob, 'state.bin')
  //   }
  // }

  useEffect(() => {
    if (animationFrameId.current != null) {
      cancelAnimationFrame(animationFrameId.current)
    }
    if (emulator != null) {
      requestAnimationFrame(execute)
    }
  }, [emulator])


  const onKeyDown = (event: KeyboardEvent) => {
    if (emulator == null) {
      return
    }
    const button = buttonMapping[event.code]
    if (button != null) {
      emulator.press_button(button)
    }
  }

  const onKeyUp = (event: KeyboardEvent) => {
    if (emulator == null) {
      return
    }
    const button = buttonMapping[event.code]
    if (button != null) {
      emulator.release_button(button)
    }
  }


  // const drawChannels = (newEmulator: WebEmulator) => () => {
  //   newEmulator.draw()
  //   requestAnimationFrame(drawChannels(newEmulator))
  // }

  const handleRomSelected = async (rom: Uint8Array) => {
    if (emulator) {
      emulator.free()
    }
    const AudioContext = window.AudioContext || window.webkitAudioContext
    const audioContext: AudioContext = new AudioContext()
    await audioContext.audioWorklet.addModule("pwm-processor.js")
    await audioContext.audioWorklet.addModule("waveform-processor.js")
    await audioContext.audioWorklet.addModule("white-noise-processor.js")
    const newEmulator = WebEmulator.new(rom, audioContext);
    setEmulator(newEmulator)
  }

  return <Fragment>
    <div className="app" onKeyDown={ onKeyDown } onKeyUp={ onKeyUp } tabIndex={ -1 }>
      <div className="title">RustBoy</div>
      <ButtonBar onRomSelected={ handleRomSelected } emulator={ emulator }/>

      {/*<div className="menu">*/ }
      {/*  <div>*/ }
      {/*    <label className="button" htmlFor="rom_selector">Choose ROM</label>*/ }
      {/*    <input*/ }
      {/*        className="hidden"*/ }
      {/*        type="file"*/ }
      {/*        id="rom_selector"*/ }
      {/*        name="rom_selector"*/ }
      {/*        accept=".gb, .gbc"*/ }
      {/*        onChange={ handleRomChange }/>*/ }
      {/*  </div>*/ }
      {/*  <div>*/ }
      {/*    <div className="button" onClick={ togglePaused }>*/ }
      {/*      { paused ? 'Resume' : 'Pause' }*/ }
      {/*    </div>*/ }
      <GameBoy emulator={ emulator }/>

      {/*<div className="audio-debugger">*/ }
      {/*  <h3>Audio</h3>*/ }
      {/*  <canvas id="ch1-canvas" width={ 200 } height={ 100 }></canvas>*/ }
      {/*  <canvas id="ch2-canvas" width={ 200 } height={ 100 }></canvas>*/ }
      {/*  <canvas id="ch3-canvas" width={ 200 } height={ 100 }></canvas>*/ }
      {/*  <canvas id="ch4-canvas" width={ 200 } height={ 100 }></canvas>*/ }
      {/*</div>*/ }
      {/*<div className="cpu-info">*/ }
      {/*  <h3>CPU Info</h3>*/ }
      {/*  {*/ }
      {/*    paused && cpuInfo != null ? <div>*/ }
      {/*      <div>AF: 0x{ cpuInfo.AF?.toString(16) }</div>*/ }
      {/*      <div>BC: 0x{ cpuInfo.BC?.toString(16) }</div>*/ }
      {/*      <div>DE: 0x{ cpuInfo.DE?.toString(16) }</div>*/ }
      {/*      <div>HL: 0x{ cpuInfo.HL?.toString(16) }</div>*/ }
      {/*      <div>SP: 0x{ cpuInfo.SP?.toString(16) }</div>*/ }
      {/*      <div>PC: 0x{ cpuInfo.PC?.toString(16) }</div>*/ }
      {/*      <div>Stopped: { cpuInfo.stopped ? 'true' : 'false' }</div>*/ }
      {/*      <div>Enabled: { cpuInfo.enabled ? 'true' : 'false' }</div>*/ }
      {/*      <div>Instruction: { instruction }</div>*/ }
      {/*    </div> : <React.Fragment/>*/ }
      {/*  }*/ }
      {/*</div>*/ }
      <TabPane emulator={ emulator }/>
    </div>
  </Fragment>


}