import {Button, Emulator, OAMObject} from '../../../pkg';
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
  const oscilloscopeRef = useRef<HTMLCanvasElement>(null)

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

  const getPWMNode = (audioContext: AudioContext, frequency: number, dutyCycle: number) => {
    const sampleRate = 8
    const audioBuffer = audioContext.createBuffer(1, 3000, 3000)
    const channel1 = audioBuffer.getChannelData(0)
    channel1.fill(-1, 0, Math.floor(3000 * dutyCycle))
    const node = audioContext.createBufferSource()
    node.buffer = audioBuffer
    node.loop = true
    const step = 1/128;
    let currentFrequency = frequency
    for(let i = 0 ; i < 512; i++) {
      currentFrequency += 2
      node.playbackRate.setValueAtTime(currentFrequency, audioContext.currentTime + (i * step))
    }
    return node
  }

  const playSound = async () => {
    const oscilloscopeCanvas = oscilloscopeRef.current
    const context2D = oscilloscopeCanvas?.getContext('2d');
    if (context2D == null) {
      return
    }
    const AudioContext = window.AudioContext || window.webkitAudioContext
    const audioContext: AudioContext = new AudioContext()

    // const channel1 = getPWMNode(audioContext, 200, 0.5)
    // // channel1.frequency.setValueAtTime(200, audioContext.currentTime)
    //
    //
    // const channel1Gain = audioContext.createGain();
    // channel1Gain.gain.setValueAtTime(0.7, audioContext.currentTime)
    // channel1Gain.gain.setValueAtTime(0.3, audioContext.currentTime + 1)

    const analyzer = audioContext.createAnalyser()
    const bufferLength = analyzer.fftSize
    const sliceWidth = 200 / bufferLength;
    const audioDataArray = new Uint8Array(bufferLength)
    let animationId: number
    const drawAudio = () => {
      context2D.clearRect(0, 0, 200, 100)
      analyzer.getByteTimeDomainData(audioDataArray)
      context2D.lineWidth = 2
      context2D.strokeStyle = "rgb(0, 0, 0)"
      context2D.beginPath()
      let x = 0
      for (let i = 0; i < bufferLength; i++) {
        const v = audioDataArray[i] / 128.0;
        const y = v * 50;

        if (i === 0) {
          context2D.moveTo(x, y);
        } else {
          context2D.lineTo(x, y);
        }
        x += sliceWidth;
      }
      context2D.lineTo(200, 50);
      context2D.stroke();
      animationId = requestAnimationFrame(drawAudio)
    }
    animationId = requestAnimationFrame(drawAudio)

    await audioContext.audioWorklet.addModule("white-noise-processor.js");
    const pwmNode = new AudioWorkletNode(
      audioContext,
      "white-noise-processor"
    );
    // pwmNode.parameters.get('dutyCycle').value = 0.25
    pwmNode.parameters.get('frequency').value = 40000
    pwmNode.parameters.get('width').value = 0.7
    const trigger = pwmNode.parameters.get('trigger');
    trigger.value = 1
    trigger.setValueAtTime(0, audioContext.currentTime + 1)
    trigger.setValueAtTime(1, audioContext.currentTime + 2)
    trigger.setValueAtTime(0, audioContext.currentTime + 3)
    trigger.setValueAtTime(1, audioContext.currentTime + 4)
    trigger.setValueAtTime(0, audioContext.currentTime + 5)
    pwmNode.connect(analyzer);
    analyzer.connect(audioContext.destination);



    // channel1.connect(channel1Gain)
    // channel1Gain.connect(analyzer)
    // analyzer.connect(audioContext.destination)
    //
    // channel1.start()
    // channel1.stop(audioContext.currentTime + 5)
    setTimeout(() => {
      cancelAnimationFrame(animationId)
      // context2D.clearRect(0, 0, 200, 100)
    }, 5000)
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
      <div className="sound-test">
        <div className="button" onClick={ playSound }>Play Sound</div>
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
      <canvas id="oscilloscope-canvas" ref={oscilloscopeRef} width={200} height={100}></canvas>
    </div>
    <div className="tile-debugger">
      <h3>Tile data</h3>
      <canvas id="tile-canvas" width={ 256 } height={ 192 }></canvas>
    </div>
  </div>
}