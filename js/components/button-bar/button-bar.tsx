import React, {FormEvent} from "react";
import {WebEmulator} from "../../../pkg/rustboy";

export interface ButtonBarProps {
  onRomSelected: (rom: Uint8Array) => void
}

export const ButtonBar = ({onRomSelected}: ButtonBarProps) => {
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

  return <div className="button-bar">
    <div className="button">
      <label htmlFor="rom_selector">Choose ROM</label>
      <input
        className="hidden"
        type="file"
        id="rom_selector"
        name="rom_selector"
        accept=".gb, .gbc"
        onChange={handleRomChange}/>
    </div>
  </div>
}