import React, {FormEvent, Fragment, useReducer} from "react";
import {saveAs} from "file-saver";

import "./button-bar.scss"
import {WebEmulator} from '../../../pkg/rustboy';

export interface ButtonBarProps {
  onRomSelected: (rom: Uint8Array) => void
  emulator: WebEmulator | undefined
}

export const ButtonBar = ({ onRomSelected, emulator }: ButtonBarProps) => {
  const [, forceUpdate] = useReducer(x => x + 1, 0);

  const handleRomChange = async (event: FormEvent<HTMLInputElement>) => {
    const files = event.currentTarget.files;
    if (files != null && files.length > 0) {
      const file = files.item(0)
      if (file != null) {
        const arrayBuffer = await file.arrayBuffer()
        const rom = new Uint8Array(arrayBuffer)
        onRomSelected(rom)
      }
    }
  }

  const handleStateChange = async (event: FormEvent<HTMLInputElement>) => {
    const files = event.currentTarget.files;
    if (files != null && files.length > 0) {
      const file = files.item(0)
      if (file != null) {
        const arrayBuffer = await file.arrayBuffer()
        const state = new Uint8Array(arrayBuffer)
        const oldPaused = emulator?.is_paused()
        emulator?.set_paused(true)
        emulator?.load_state(state)
        emulator?.set_paused(false)
      }
    }
  }

  const togglePaused = () => {
    emulator?.set_paused(!emulator?.is_paused())
    forceUpdate()
  }

  const saveState = () => {
    if (emulator) {
      emulator.set_paused(true)
      const state = emulator.get_state()
      const blob = new Blob([state.buffer], {
        type: 'application/octet-stream'
      })
      saveAs(blob, 'state.bin')
      emulator.set_paused(false)
    }
  }

  const SaveStateButton = () => {
    if (emulator == null) {
      return <Fragment/>
    }
    return <div className="button" onClick={ saveState }>Save State</div>
  }

  const LoadStateButton = () => {
    if (emulator == null) {
      return <Fragment/>
    }
    return <div className="button">
      <label htmlFor="state_selector">Load State</label>
      <input
        className="hidden"
        type="file"
        id="state_selector"
        name="state_selector"
        accept=".bin"
        onChange={ handleStateChange }/>
    </div>
  }

  const PauseButton = () => {
    if (emulator == null) {
      return <Fragment/>
    }
    return <div className="button" onClick={ togglePaused }>
      { emulator.is_paused() ? 'Resume' : 'Pause' }
    </div>
  }

  return <div className="button-bar">
    <div className="button">
      <label htmlFor="rom_selector">Choose ROM</label>
      <input
        className="hidden"
        type="file"
        id="rom_selector"
        name="rom_selector"
        accept=".gb, .gbc"
        onChange={ handleRomChange }/>
    </div>
    <SaveStateButton/>
    <LoadStateButton/>
    <PauseButton/> `
  </div>
}