import React, {FormEvent, Fragment, useState} from "react";

import "./button-bar.scss"
import {WebEmulator} from "../../../pkg/rustboy";

export interface ButtonBarProps {
  onRomSelected: (rom: Uint8Array) => void
  emulator: WebEmulator | undefined
}

export const ButtonBar = ({ onRomSelected, emulator }: ButtonBarProps) => {
  const handleRomChange = async (event: FormEvent<HTMLInputElement>) => {
    const files = event.currentTarget.files;
    if (files != null && files.length > 0) {
      const file = files.item(0)
      if (file != null) {
        const arrayBuffer = await file.arrayBuffer()
        const rom = new Uint8Array(arrayBuffer);
        onRomSelected(rom)
      }
    }
  }

  const togglePaused = () => {
    emulator?.set_paused(emulator?.is_paused())
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
    <PauseButton/>        `
  </div>
}