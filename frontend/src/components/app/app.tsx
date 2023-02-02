import {Button, Emulator, OAMObject} from '../../../rustboy';
import React, {FormEvent, KeyboardEvent, MouseEvent, useEffect, useRef, useState} from 'react'
import './app.scss'
// @ts-ignore
import gbImage from '../../images/gb.png'

export const App = () => {

  const [emulator, setEmulator] = useState<Emulator>()
  const [paused, setPaused] = useState<boolean>(false)
  const [objectInfoIndex, setObjectInfoIndex] = useState<number>()
  const [selectedObject, setSelectedObject] = useState<OAMObject>()
  const pausedRef = useRef<boolean>(false)
  const previousTimeRef = useRef<number>()
  const animationFrameId = useRef<number>()

  const execute = () => {
    let currentTime = performance.now();
    let delta_ms = previousTimeRef.current ? (currentTime - previousTimeRef.current) : 1
    if (!pausedRef.current) {
      emulator?.tick(BigInt(Math.floor(delta_ms * 1_000_000)))
    }
    previousTimeRef.current = currentTime
    animationFrameId.current = requestAnimationFrame(execute)
  }

  const togglePaused = () => {
    setPaused(!paused)
    pausedRef.current = !pausedRef.current
  }

  useEffect(() => {
    if (animationFrameId.current != null) {
      cancelAnimationFrame(animationFrameId.current)
    }
    if (emulator != null) {
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
    switch (event.code) {
      case 'KeyW': {
        emulator.press_button(Button.UP)
        break
      }
      case 'KeyA': {
        emulator.press_button(Button.LEFT)
        break
      }
      case 'KeyS': {
        emulator.press_button(Button.DOWN)
        break
      }
      case 'KeyD': {
        emulator.press_button(Button.RIGHT)
        break
      }
      case 'KeyB': {
        emulator.press_button(Button.A)
        break
      }
      case 'KeyN': {
        emulator.press_button(Button.B)
        break
      }
      case 'KeyT': {
        emulator.press_button(Button.START)
        break
      }
      case 'KeyY': {
        emulator.press_button(Button.SELECT)
        break
      }
    }
  }

  const onKeyUp = (event: KeyboardEvent) => {
    if (emulator == null) {
      return
    }
    switch (event.code) {
      case 'KeyW': {
        emulator.release_button(Button.UP)
        break
      }
      case 'KeyA': {
        emulator.release_button(Button.LEFT)
        break
      }
      case 'KeyS': {
        emulator.release_button(Button.DOWN)
        break
      }
      case 'KeyD': {
        emulator.release_button(Button.RIGHT)
        break
      }
      case 'KeyB': {
        emulator.release_button(Button.A)
        break
      }
      case 'KeyN': {
        emulator.release_button(Button.B)
        break
      }
      case 'KeyT': {
        emulator.release_button(Button.START)
        break
      }
      case 'KeyY': {
        emulator.release_button(Button.SELECT)
        break
      }
    }
  }

  const onMouseMoveInObjectCanvas = (event: MouseEvent) => {
    const infoElement = event.currentTarget
    const infoElementStyle = window.getComputedStyle(infoElement)
    const infoElementBoundingRect = infoElement.getBoundingClientRect()
    const x = Math.round(event.clientX - infoElementBoundingRect.left - parseFloat(infoElementStyle.borderLeftWidth ?? '0'))
    const y = Math.round(event.clientY - infoElementBoundingRect.top - parseFloat(infoElementStyle.borderTopWidth ?? '0'))
    if (x >= 0 && x <= 159 && y >= 0 && y<= 31) {
      const objectIndex = Math.floor(y / 16) * 16 + Math.floor(x/8)
      setObjectInfoIndex(objectIndex)
    } else {
      setObjectInfoIndex(undefined)
    }
  }

  const onMouseLeaveObjectCanvas = () => {
    setObjectInfoIndex(undefined)
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
        setEmulator(Emulator.new(byteArray))
      }
    }
  }

  return <div className="app" onKeyDown={ onKeyDown } onKeyUp={ onKeyUp } tabIndex={ 0 }>
    <h1 className="title">RustBoy</h1>
    <div className="menu">
      <div className="rom-selection">
        <label className="button" htmlFor="rom_selector">Choose ROM</label>
        <input
          className="hidden"
          type="file"
          id="rom_selector"
          name="rom_selector"
          accept=".gb, .gbc"
          onChange={ handleRomChange }/>
      </div>
      <div className="playback-control">
        <div className="button" onClick={ togglePaused }>
          { paused ? 'Resume' : 'Pause' }
        </div>
      </div>
    </div>
    <div className="gameboy">
      <img width={ "361px" } height={ "621px" } src={ gbImage }></img>
      <canvas id="main-canvas" width={ 160 } height={ 144 }></canvas>
    </div>
    <div className="object-debugger">
      <h3>OAM Content</h3>
      <canvas id="object-canvas" onMouseMove={ onMouseMoveInObjectCanvas } width={ 160 } height={ 32 }></canvas>
      {
        selectedObject ? <div id="object-info-container">
          <div>X: { selectedObject.lcd_x }</div>
          <div>Y: { selectedObject.lcd_y }</div>
          <div>Tile Index: { selectedObject.tile_index }</div>
          <div>Attributes: { `0x${selectedObject.attributes.value().toString(16)}` }</div>
        </div>  : <React.Fragment/>

      }
    </div>
    <div className="tile-debugger">
      <h3>Tile data</h3>
      <canvas id="tile-canvas" width={ 256 } height={ 192 }></canvas>
    </div>
  </div>
}