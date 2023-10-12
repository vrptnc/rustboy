import {Button, CPUInfo, OAMObject, WebEmulator} from '../../../pkg/rustboy';
import React, {FormEvent, KeyboardEvent, MouseEvent, useEffect, useRef, useState} from 'react'
import './app.scss'
// @ts-ignore
import gbImage from '../../images/gb.png'

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

  const getMouseDownHandler = (button: Button) => () => {
    emulator?.press_button(button)
  }

  const getMouseUpHandler = (button: Button) => () => {
    emulator?.release_button(button)
  }

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

  const drawChannels = (newEmulator: Emulator) => () => {
    newEmulator.draw()
    requestAnimationFrame(drawChannels(newEmulator))
  }

  const handleRomChange = async (event: FormEvent<HTMLInputElement>) => {
    const files = event.currentTarget.files;
    if (files != null && files.length > 0) {
      const file = files.item(0)
      if (file != null) {
        const arrayBuffer = await file.arrayBuffer()
        const byteArray = new Uint8Array(arrayBuffer);
        if (emulator) {
          emulator.free()
        }
        const AudioContext = window.AudioContext || window.webkitAudioContext
        const audioContext: AudioContext = new AudioContext()
        await audioContext.audioWorklet.addModule("pwm-processor.js")
        await audioContext.audioWorklet.addModule("waveform-processor.js")
        await audioContext.audioWorklet.addModule("white-noise-processor.js")
        const newEmulator = WebEmulator.new(byteArray, audioContext);
        setEmulator(newEmulator)
        requestAnimationFrame(drawChannels(newEmulator))
      }
    }
  }

  return <div className="app" onKeyDown={ onKeyDown } onKeyUp={ onKeyUp } tabIndex={ 0 }>
    <div className="title">RustBoy</div>
    <div className="button-bar">
      <div>
        <label className="button" htmlFor="rom_selector">Choose ROM</label>
        <input
          className="hidden"
          type="file"
          id="rom_selector"
          name="rom_selector"
          accept=".gb, .gbc"
          onChange={ handleRomChange }/>
      </div>
    </div>
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
    <div className="gameboy">
      <canvas id="main-canvas" width={ 160 } height={ 144 }></canvas>
      <div className="control-panel">
        <div id="up-button" onMouseDown={ getMouseDownHandler(Button.UP) }
             onMouseUp={ getMouseUpHandler(Button.UP) }></div>
        <div id="down-button" onMouseDown={ getMouseDownHandler(Button.DOWN) }
             onMouseUp={ getMouseUpHandler(Button.DOWN) }></div>
        <div id="left-button" onMouseDown={ getMouseDownHandler(Button.LEFT) }
             onMouseUp={ getMouseUpHandler(Button.LEFT) }></div>
        <div id="right-button" onMouseDown={ getMouseDownHandler(Button.RIGHT) }
             onMouseUp={ getMouseUpHandler(Button.RIGHT) }></div>
        <div id="center-button"></div>
        <div className="action-panel">
          <div id="a-button" className="action-button" onMouseDown={ getMouseDownHandler(Button.A) }
               onMouseUp={ getMouseUpHandler(Button.A) }>
            <div className="label">A</div>
          </div>
          <div id="b-button" className="action-button" onMouseDown={ getMouseDownHandler(Button.B) }
               onMouseUp={ getMouseUpHandler(Button.B) }>
            <div className="label">B</div>
          </div>
        </div>
      </div>
    </div>

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
  </div>
}