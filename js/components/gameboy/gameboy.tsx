import {ControlPanel} from "../control-panel/control-panel";
import React, {useEffect, useRef, useState} from "react";
import {WebEmulator} from "../../../pkg/rustboy";
import "./gameboy.scss"

export interface GameBoyProps {
  emulator: WebEmulator | undefined
}

export const GameBoy = ({emulator}: GameBoyProps) => {
  return <div className="gameboy">
    <canvas id="main-canvas" width={160} height={144}></canvas>
    <ControlPanel emulator={emulator}></ControlPanel>
  </div>
}