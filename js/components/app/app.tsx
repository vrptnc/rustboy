import {Button, CPUInfo, OAMObject, WebEmulator} from '../../../pkg/rustboy';
import React, {FormEvent, Fragment, KeyboardEvent, MouseEvent, useEffect, useRef, useState} from 'react'
import {ControlPanel} from "../control-panel/control-panel";
import './app.scss'
// @ts-ignore
import gbImage from '../../images/gb.png'
import {GameBoy} from "../gameboy/gameboy";
import {ButtonBar} from "../button-bar/button-bar";
import {TabPane} from "../tab-pane/tab-pane";

export const App = () => {

  const [emulator, setEmulator] = useState<WebEmulator>()
  const [paused, setPaused] = useState<boolean>(false)
  const [objectInfoIndex, setObjectInfoIndex] = useState<number>()
  const [selectedObject, setSelectedObject] = useState<OAMObject>()
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

  const togglePaused = () => {
    if (paused) {
      scheduleRun()
    } else if (animationFrameId.current != null) {
      cancelAnimationFrame(animationFrameId.current)
      previousTimeRef.current = undefined
      const info = emulator?.cpu_info();
      setCPUInfo(info)
    }
    setPaused(!paused)
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
    if (emulator != null && !paused) {
      requestAnimationFrame(execute)
    }
  }, [emulator])

  useEffect(() => {
    if (objectInfoIndex != null && emulator != null) {
      const object = emulator.get_object(objectInfoIndex)
      setSelectedObject(object)
    } else {
      setSelectedObject(undefined)
    }

  }, [objectInfoIndex])



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

  const onMouseMoveInObjectCanvas = (event: MouseEvent) => {
    const infoElement = event.currentTarget
    const infoElementStyle = window.getComputedStyle(infoElement)
    const infoElementBoundingRect = infoElement.getBoundingClientRect()
    const x = Math.round(event.clientX - infoElementBoundingRect.left - parseFloat(infoElementStyle.borderLeftWidth ?? '0'))
    const y = Math.round(event.clientY - infoElementBoundingRect.top - parseFloat(infoElementStyle.borderTopWidth ?? '0'))
    if (x >= 0 && x <= 159 && y >= 0 && y <= 31) {
      const objectIndex = Math.floor(y / 16) * 16 + Math.floor(x / 8)
      setObjectInfoIndex(objectIndex)
    } else {
      setObjectInfoIndex(undefined)
    }
  }

  const onMouseLeaveObjectCanvas = () => {
    setObjectInfoIndex(undefined)
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
      <ButtonBar onRomSelected={handleRomSelected}/>

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
      {/*  </div>*/ }
      {/*</div>*/ }
      {/*<canvas id="object-canvas" onMouseMove={ onMouseMoveInObjectCanvas } onMouseLeave={ onMouseLeaveObjectCanvas }*/ }
      {/*        width={ 160 } height={ 32 }></canvas>*/ }
      {/*<canvas id="tile-canvas" width={ 256 } height={ 192 }></canvas>*/ }
      <GameBoy emulator={emulator}/>

      {/*<div className="gameboy">*/ }
      {/*  <img width={ "361px" } height={ "621px" } src={ gbImage }></img>*/ }
      {/*  <canvas id="main-canvas" width={ 160 } height={ 144 }></canvas>*/ }
      {/*</div>*/ }
      {/*<div className="object-debugger">*/ }
      {/*  <h3>OAM Content</h3>*/ }
      {/*  <canvas id="object-canvas" onMouseMove={ onMouseMoveInObjectCanvas } onMouseLeave={ onMouseLeaveObjectCanvas }*/ }
      {/*          width={ 160 } height={ 32 }></canvas>*/ }
      {/*  {*/ }
      {/*    selectedObject ? <div id="object-info-container">*/ }
      {/*      <div>X: { selectedObject.lcd_x }</div>*/ }
      {/*      <div>Y: { selectedObject.lcd_y }</div>*/ }
      {/*      <div>Tile Index: { selectedObject.tile_index }</div>*/ }
      {/*      <div>Attributes: { `0x${ selectedObject.attributes.value().toString(16) }` }</div>*/ }
      {/*    </div> : <React.Fragment/>*/ }

      {/*  }*/ }
      {/*</div>*/ }
      {/*<div className="tile-debugger">*/ }
      {/*  <h3>Tile data</h3>*/ }
      {/*  <canvas id="tile-canvas" width={ 256 } height={ 192 }></canvas>*/ }
      {/*</div>*/ }
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
      <TabPane emulator={emulator}/>
    </div>
  </Fragment>


}